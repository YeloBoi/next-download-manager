package io.github.yeloboi.ndm;

import android.app.*;
import android.content.*;
import android.content.pm.PackageInstaller;
import android.os.Build;

public final class InstallResult extends BroadcastReceiver {
    static volatile Event latest;
    record Event(int status,String name,String message,Intent confirm){}
    @Override public void onReceive(Context context,Intent intent){
        if(!"io.github.yeloboi.ndm.INSTALL_RESULT".equals(intent.getAction()))return;
        int session=intent.getIntExtra(PackageInstaller.EXTRA_SESSION_ID,-1);
        SharedPreferences saved=context.getSharedPreferences("installer",Context.MODE_PRIVATE);
        String name=saved.getString("package_"+session,null);
        if(name==null)return;
        int status=intent.getIntExtra(PackageInstaller.EXTRA_STATUS,PackageInstaller.STATUS_FAILURE);
        Intent confirm=Build.VERSION.SDK_INT>=33?intent.getParcelableExtra(Intent.EXTRA_INTENT,Intent.class):intent.getParcelableExtra(Intent.EXTRA_INTENT);
        latest=new Event(status,name,intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE),confirm);
        CompanionClient client=CompanionClient.get(context);
        if(status==PackageInstaller.STATUS_PENDING_USER_ACTION&&confirm!=null){
            if(client.foreground)return;
            confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            PendingIntent pending=PendingIntent.getActivity(context,session,confirm,PendingIntent.FLAG_IMMUTABLE|PendingIntent.FLAG_UPDATE_CURRENT);
            Notification n=new Notification.Builder(context,CompanionClient.EVENTS).setSmallIcon(android.R.drawable.stat_sys_download)
                .setContentTitle("Confirm APK installation").setContentText(name).setContentIntent(pending).setAutoCancel(true).build();
            try{context.getSystemService(NotificationManager.class).notify(session,n);}catch(SecurityException ignored){}
        }else{
            saved.edit().remove("package_"+session).apply();
            if(!client.foreground)client.notifyEvent(status==PackageInstaller.STATUS_SUCCESS?"App installed":"Installation did not finish",name,session);
        }
    }
}
