"""Exercise release packages against real browsers on an isolated GitHub Windows runner."""
import hashlib
import json
import os
from pathlib import Path
import socketserver
import subprocess
import tempfile
import threading
import time
import uuid
import winreg
import zipfile
from http.server import BaseHTTPRequestHandler

from selenium import webdriver
from selenium.webdriver.chrome.service import Service as ChromeService
from selenium.webdriver.common.by import By
from selenium.webdriver.edge.service import Service as EdgeService
from selenium.webdriver.firefox.service import Service as FirefoxService
from selenium.webdriver.support.ui import WebDriverWait

assert os.environ.get('GITHUB_ACTIONS') == 'true'
CHROMIUM_ID = 'llccemgchkhblimpddegikfekglcjoen'
FIREFOX_ID = 'ndm@yeloboi.github.io'
ARTIFACTS = Path('.artifacts').resolve()
DATA = bytes(range(251)) * 512
DOWNLOADS = Path(os.environ['USERPROFILE']) / 'Downloads'
STARTUP_LOG = Path(os.environ['LOCALAPPDATA']) / 'NDM' / 'browser-startup-error.log'
RECEIVED = set()


class Files(BaseHTTPRequestHandler):
    def do_GET(self):
        RECEIVED.add(self.path)
        self.send_response(200)
        self.send_header('Content-Type', 'application/octet-stream')
        self.send_header('Content-Length', str(len(DATA)))
        self.end_headers()
        self.wfile.write(DATA)

    def log_message(self, *_):
        pass


def browser_session(name, chromium, firefox, log):
    if name == 'Firefox':
        options = webdriver.FirefoxOptions()
        options.add_argument('--allow-system-access')
        driver = webdriver.Firefox(options=options, service=FirefoxService(
            str(Path(os.environ['GECKOWEBDRIVER']) / 'geckodriver.exe')))
        try:
            assert driver.install_addon(str(firefox), temporary=True) == FIREFOX_ID
            driver.set_context('chrome')
            popup = driver.execute_script("return WebExtensionPolicy.getByID(arguments[0]).getURL('popup.html')", FIREFOX_ID)
            driver.set_context('content')
            return driver, popup
        except BaseException:
            driver.quit()
            raise
    chrome = name == 'Chrome'
    options = webdriver.ChromeOptions() if chrome else webdriver.EdgeOptions()
    options.enable_bidi = True
    options.enable_webextensions = True
    options.add_argument('--headless=new')
    options.add_argument('--lang=en-US')
    options.add_argument('--enable-logging')
    options.add_argument(f'--log-file={log}')
    options.add_argument('--vmodule=launch_context*=1')
    service_class = ChromeService if chrome else EdgeService
    binary = 'chromedriver.exe' if chrome else 'msedgedriver.exe'
    service = service_class(str(Path(os.environ[f'{name.upper()}WEBDRIVER']) / binary))
    driver = (webdriver.Chrome if chrome else webdriver.Edge)(options=options, service=service)
    try:
        extension = driver.install_web_extension(str(chromium))
        assert extension.id == CHROMIUM_ID, extension.id
        return driver, f'chrome-extension://{extension.id}/popup.html'
    except BaseException:
        driver.quit()
        raise


def popup_download(driver, url, action):
    driver.find_element(By.ID, 'urls').send_keys(url)
    driver.find_element(By.ID, action).click()
    try:
        def result(d):
            status = d.find_element(By.ID, 'result').text
            if 'Links are saved' in status and d.find_element(By.ID, action).is_enabled():
                raise AssertionError(status)
            return 'Accepted by NDM' in status and d.find_element(By.ID, action).is_enabled()
        WebDriverWait(driver, 40).until(result)
    except Exception as error:
        raise AssertionError(f"{action} failed: {driver.find_element(By.ID, 'result').text}; "
                             f"{driver.find_element(By.ID, 'connection').text}") from error


def verify_file(filename):
    file = DOWNLOADS / filename
    deadline = time.monotonic() + 30
    while not file.is_file() and time.monotonic() < deadline:
        time.sleep(0.2)
    assert file.is_file() and hashlib.sha256(file.read_bytes()).digest() == hashlib.sha256(DATA).digest(), filename


def native_processes():
    processes = subprocess.run(['tasklist', '/fo', 'csv'], capture_output=True, text=True, timeout=10, check=True)
    return [line for line in processes.stdout.splitlines()
            if line.lower().startswith(('"ndm.exe"', '"ndm-native-host.exe"'))]


def stop_desktop(force=False):
    args = ['taskkill', '/im', 'ndm.exe']
    subprocess.run(args + (['/f'] if force else []), stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL, timeout=15, check=False)
    deadline = time.monotonic() + 12
    while any(line.lower().startswith('"ndm.exe"') for line in native_processes()):
        assert time.monotonic() < deadline, 'Desktop did not shut down'
        time.sleep(0.25)


def native_probe(driver, kind='ping', url=None):
    return driver.execute_async_script("""
      const [kind, url, done] = arguments;
      const api = globalThis.browser ?? globalThis.chrome;
      const message = {version:1, request_id:'runner-probe', kind, urls:url?[url]:[],
                       start:kind==='add', created_at:Math.floor(Date.now()/1000)};
      api.runtime.sendNativeMessage('local.ndm.bridge',message).then(reply=>done({reply}),error=>done({error:String(error)}));
    """, kind, url)


def assert_registered(install):
    for name in ('Google\\Chrome', 'Microsoft\\Edge', 'Mozilla'):
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, f'Software\\{name}\\NativeMessagingHosts\\local.ndm.bridge') as key:
            manifest = json.loads(Path(winreg.QueryValueEx(key, '')[0]).read_bytes())
        assert Path(manifest['path']) == install / 'ndm-native-host.exe', manifest
        if name == 'Mozilla':
            assert manifest['allowed_extensions'] == [FIREFOX_ID]
        else:
            assert f'chrome-extension://{CHROMIUM_ID}/' in manifest['allowed_origins']


def check(name, chromium, firefox, base, log):
    STARTUP_LOG.unlink(missing_ok=True)
    driver, popup = browser_session(name, chromium, firefox, log)
    try:
        print(f'{name} {driver.capabilities["browserVersion"]}: loaded release extension', flush=True)
        driver.set_script_timeout(45)
        driver.get(popup)
        wait = WebDriverWait(driver, 25)
        wait.until(lambda d: 'Waiting for NDM' in d.find_element(By.ID, 'connection').text)
        probe = native_probe(driver)
        print(f'{name} native probe with NDM closed: {probe}', flush=True)
        assert probe.get('reply', {}).get('request_id') == 'runner-probe', probe
        assert probe['reply']['accepted'] is False, probe
        cold = f'{name}-cold-{uuid.uuid4().hex}.bin'
        try:
            popup_download(driver, f'{base}/{cold}', 'now')
        except AssertionError as error:
            print(f'{name} popup error: {error}; HTTP request observed: {f"/{cold}" in RECEIVED}', flush=True)
            for process in native_processes():
                print(f'Native processes: {process}', flush=True)
            if STARTUP_LOG.is_file():
                print(STARTUP_LOG.read_text(encoding='utf-8', errors='replace'), flush=True)
            try:
                print(f'{name} direct download probe: {native_probe(driver, "add", f"{base}/{cold}")}', flush=True)
            except Exception as probe_error:
                print(f'{name} direct download probe failed: {probe_error}', flush=True)
            raise
        verify_file(cold)
        wait.until(lambda d: d.find_element(By.ID, 'connection').text == 'Desktop connected')
        queued = f'{name}-queued-{uuid.uuid4().hex}.bin'
        # The app queue is still running after cold delivery. Stop/reopen to check queue-only semantics.
        stop_desktop()
        wait.until(lambda d: 'Waiting for NDM' in d.find_element(By.ID, 'connection').text)
        popup_download(driver, f'{base}/{queued}', 'queue')
        time.sleep(1)
        assert f'/{queued}' not in RECEIVED, 'Adding to a stopped queue must not fetch a file'
        popup_download(driver, f'{base}/{queued}', 'now')
        verify_file(queued)
        wait.until(lambda d: not d.find_elements(By.CSS_SELECTOR, '#selection li details'))
        print(f'{name} passed: installer registration, actual offline cold launch, automatic connection, queue-only, start existing queue item and complete files.', flush=True)
    finally:
        stop_desktop()
        driver.quit()


with tempfile.TemporaryDirectory(prefix='ndm-extensions-', ignore_cleanup_errors=True) as temporary:
    root = Path(temporary)
    install = root / 'installed'
    setup, = ARTIFACTS.glob('*-setup.exe')
    firefox, = ARTIFACTS.glob('*-firefox.zip')
    chromium_zip, = ARTIFACTS.glob('*-chromium.zip')
    chromium = root / 'chromium'
    with zipfile.ZipFile(chromium_zip) as archive:
        archive.extractall(chromium)
    # NSIS treats the final /D= argument as the entire path, without quotes.
    subprocess.run(f'"{setup}" /S /D={install}', timeout=120, check=True)
    try:
        assert_registered(install)
        stop_desktop()
        with socketserver.ThreadingTCPServer(('127.0.0.1', 0), Files) as server:
            threading.Thread(target=server.serve_forever, daemon=True).start()
            base = f'http://127.0.0.1:{server.server_address[1]}'
            try:
                failures = []
                for name in ('Firefox', 'Chrome', 'Edge'):
                    log = root / f'{name}.log'
                    try:
                        check(name, chromium, firefox, base, log)
                    except Exception as error:
                        failures.append((name, str(error)))
                        print(f'{name} failed: {error}', flush=True)
                        if log.is_file():
                            print('\n'.join(line for line in log.read_text(encoding='utf-8', errors='replace').splitlines()
                                            if 'native messaging' in line.lower() or 'launch_context' in line.lower()), flush=True)
                assert not failures, failures
            finally:
                server.shutdown()
    finally:
        stop_desktop(force=True)
        subprocess.run(['taskkill', '/im', 'ndm-native-host.exe', '/f'], stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=15, check=False)
        subprocess.run([str(install / 'uninstall.exe'), '/S'], timeout=60, check=True)
