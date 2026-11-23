package io.github.yeloboi.ndm;

import android.app.PendingIntent;
import android.content.*;
import android.content.pm.*;
import android.graphics.drawable.Drawable;
import android.net.Uri;
import android.os.Build;
import java.io.*;
import java.util.UUID;
import java.util.function.IntConsumer;

final class ApkInstall {
    record Details(File file,String label,String packageName,String version,long size,Drawable icon){}
    static Details inspect(Context context,Uri uri)throws Exception {
        if(!"content".equals(uri.getScheme()))throw new IOException("Select an APK from your device.");
        File path=new File(context.getCacheDir(),"install-"+UUID.randomUUID()+".apk");
        try{
            PhoneDownloads.copyUri(context.getContentResolver(),uri,path,4L*1024*1024*1024);
            return details(context,path);
        }catch(Exception error){path.delete();throw error;}
    }
    static Details details(Context context,File path)throws IOException{
        PackageManager pm=context.getPackageManager();
        PackageInfo pkg=pm.getPackageArchiveInfo(path.getAbsolutePath(),0);
        if(pkg==null||pkg.applicationInfo==null)throw new IOException("This is not a valid standalone APK.");
        ApplicationInfo info=pkg.applicationInfo;
        info.sourceDir=path.getAbsolutePath();info.publicSourceDir=path.getAbsolutePath();
        return new Details(path,String.valueOf(info.loadLabel(pm)),pkg.packageName,pkg.versionName!=null?pkg.versionName:"Unknown",path.length(),info.loadIcon(pm));
    }
    static int stage(Context context,Details details,IntConsumer progress)throws Exception{
        PackageInstaller installer=context.getPackageManager().getPackageInstaller();
        PackageInstaller.SessionParams params=new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
        params.setAppPackageName(details.packageName());params.setSize(details.size());
        if(Build.VERSION.SDK_INT>=31)params.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_REQUIRED);
        int id=installer.createSession(params);
        if(!context.getSharedPreferences("installer",Context.MODE_PRIVATE).edit().putString("package_"+id,details.packageName()).commit()){
            installer.abandonSession(id);
            throw new IOException("Could not save the installation session.");
        }
        try(PackageInstaller.Session session=installer.openSession(id)){
            try(InputStream source=new FileInputStream(details.file());OutputStream output=session.openWrite("base.apk",0,details.size())){
                byte[] buffer=new byte[65536];long sent=0;int n;int previous=-1;
                while((n=source.read(buffer))!=-1){
                    output.write(buffer,0,n);sent+=n;
                    int percentage=(int)Math.min(95,sent*95/Math.max(1,details.size()));
                    if(percentage!=previous){progress.accept(percentage);session.setStagingProgress(percentage/100f);previous=percentage;}
                }
                session.fsync(output);
            }
            int flags=PendingIntent.FLAG_UPDATE_CURRENT;
            if(Build.VERSION.SDK_INT>=31)flags|=PendingIntent.FLAG_MUTABLE;
            Intent intent=new Intent(context,InstallResult.class).setAction("io.github.yeloboi.ndm.INSTALL_RESULT");
            PendingIntent receiver=PendingIntent.getBroadcast(context,id,intent,flags);
            session.commit(receiver.getIntentSender());
        }catch(Exception error){installer.abandonSession(id);throw error;}
        finally{details.file().delete();}
        return id;
    }
}
