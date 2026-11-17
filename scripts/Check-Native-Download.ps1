param([Parameter(Mandatory)][string]$UrlBase, [Parameter(Mandatory)][string]$ExpectedHash)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'This check may run only on an isolated GitHub Windows runner.' }
$desktopPath = (Resolve-Path '.artifacts/portable/ndm.exe').Path
$hostPath = (Resolve-Path '.artifacts/portable/ndm-native-host.exe').Path
./scripts/Register-NativeHost.ps1 -Browser Firefox -HostPath $hostPath
$manifestPath = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'NDM/bridge/local.ndm.bridge.firefox.json'

# Model Firefox's native-host job, including killing it immediately after the reply.
Add-Type -TypeDefinition @'
using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
public sealed class NdmNativeSession : IDisposable {
    [StructLayout(LayoutKind.Sequential)] struct BasicLimits {
        public long ProcessTime, JobTime;
        public uint Flags;
        public UIntPtr MinimumWorkingSet, MaximumWorkingSet;
        public uint ActiveProcessLimit;
        public UIntPtr Affinity;
        public uint PriorityClass, SchedulingClass;
    }
    [StructLayout(LayoutKind.Sequential)] struct IoCounters {
        public ulong ReadCount, WriteCount, OtherCount, ReadBytes, WriteBytes, OtherBytes;
    }
    [StructLayout(LayoutKind.Sequential)] struct ExtendedLimits {
        public BasicLimits Basic;
        public IoCounters Io;
        public UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
    }
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr CreateJobObject(IntPtr attributes, string name);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job, int type, ref ExtendedLimits limits, uint size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    IntPtr job;
    Process host;
    static void Require(bool result) {
        if (!result) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
    }
    public NdmNativeSession(string executable, string manifest) {
        try {
            job = CreateJobObject(IntPtr.Zero, null);
            Require(job != IntPtr.Zero);
            var limits = new ExtendedLimits();
            limits.Basic.Flags = 0x2000 | 0x0800; // KILL_ON_JOB_CLOSE | BREAKAWAY_OK
            Require(SetInformationJobObject(job, 9, ref limits, (uint)Marshal.SizeOf<ExtendedLimits>()));
            var start = new ProcessStartInfo(executable) {
                UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true
            };
            start.ArgumentList.Add(manifest);
            start.ArgumentList.Add("ndm@yeloboi.github.io");
            host = Process.Start(start);
            Require(AssignProcessToJobObject(job, host.Handle));
        } catch { Dispose(); throw; }
    }
    static byte[] ReadBytes(Stream input, int count) {
        var bytes = new byte[count];
        int offset = 0;
        while (offset < count) {
            int read = input.Read(bytes, offset, count - offset);
            if (read == 0) throw new EndOfStreamException("Native host exited without a complete reply.");
            offset += read;
        }
        return bytes;
    }
    public string Send(string json) {
        byte[] bytes = Encoding.UTF8.GetBytes(json);
        Stream input = host.StandardInput.BaseStream;
        input.Write(BitConverter.GetBytes(bytes.Length));
        input.Write(bytes);
        input.Flush();
        var reply = Task.Run(() => {
            Stream output = host.StandardOutput.BaseStream;
            int length = BitConverter.ToInt32(ReadBytes(output, 4));
            if (length < 1 || length > 256 * 1024) throw new InvalidDataException("Invalid native frame.");
            return Encoding.UTF8.GetString(ReadBytes(output, length));
        });
        if (!reply.Wait(TimeSpan.FromSeconds(40))) throw new TimeoutException("Native reply timed out.");
        return reply.Result;
    }
    public void Dispose() {
        if (job != IntPtr.Zero) { CloseHandle(job); job = IntPtr.Zero; }
        host?.Dispose();
    }
}
'@
function Get-Desktop { @(Get-Process -Name ndm -ErrorAction SilentlyContinue | Where-Object Path -eq $desktopPath) }
function Wait-Until([scriptblock]$Condition, [string]$Description) {
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    do {
        if (& $Condition) { return }
        Start-Sleep -Milliseconds 150
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out: $Description"
}
function Send-Native([string]$Kind, [string[]]$Urls = @()) {
    $request = @{ version=1; request_id=[guid]::NewGuid().ToString(); kind=$Kind; urls=@($Urls); start=($Kind -eq 'add'); created_at=[DateTimeOffset]::UtcNow.ToUnixTimeSeconds() }
    $session = [NdmNativeSession]::new($hostPath, $manifestPath)
    try { $response = $session.Send(($request | ConvertTo-Json -Compress)) | ConvertFrom-Json }
    finally { $session.Dispose() }
    if ($response.request_id -ne $request.request_id) { throw 'Native response correlation failed.' }
    return $response
}
function Assert-Visible {
    $processes = @(Get-Desktop)
    return $processes.Count -eq 1 -and $processes[0].MainWindowHandle -ne 0 -and [NdmNativeSession]::IsWindowVisible($processes[0].MainWindowHandle) -and -not [NdmNativeSession]::IsIconic($processes[0].MainWindowHandle)
}
try {
    if (@(Get-Desktop).Count -ne 0) { throw 'Cold-launch check requires no existing desktop.' }
    $offline = Send-Native 'ping'
    if ($offline.accepted -or @(Get-Desktop).Count -ne 0) { throw 'A background ping must not launch NDM.' }
    foreach ($phase in @('cold', 'warm')) {
        if ($phase -eq 'warm') {
            $online = Send-Native 'ping'
            if (-not $online.accepted) { throw 'Desktop must be connected before warm download.' }
            $originalId = (Get-Desktop)[0].Id
            $window = (Get-Desktop)[0].MainWindowHandle
            [void][NdmNativeSession]::ShowWindow($window, 6)
            [void][NdmNativeSession]::ShowWindow($window, 0)
        }
        $filename = "ndm-$phase-$([guid]::NewGuid().ToString('N')).bin"
        $response = Send-Native 'add' @("$UrlBase/$filename")
        if (-not $response.accepted) { throw "Desktop rejected download: $($response.error)" }
        Wait-Until { Assert-Visible } "$phase download must show one restored NDM window after native-host job teardown"
        $file = Join-Path ([Environment]::GetFolderPath('UserProfile')) "Downloads/$filename"
        Wait-Until { Test-Path -LiteralPath $file } "$phase download must finish after the host is killed"
        if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $ExpectedHash) { throw 'Downloaded bytes differ from the HTTP fixture.' }
        if ($phase -eq 'warm' -and (Get-Desktop)[0].Id -ne $originalId) { throw 'Warm request must reuse the existing desktop.' }
        Write-Output "$phase native download passed: visible desktop, acknowledged request, complete SHA-256 verified file."
    }
} finally {
    Get-Desktop | Stop-Process -Force -ErrorAction SilentlyContinue
}
