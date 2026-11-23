package io.github.yeloboi.ndm;

import android.app.*;
import android.content.Intent;
import android.os.IBinder;

public final class CompanionService extends Service {
    @Override public int onStartCommand(Intent intent,int flags,int id) {
        CompanionClient client=CompanionClient.get(this);
        if(client.pc==null){stopSelf();return START_NOT_STICKY;}
        Notification notification=new Notification.Builder(this,CompanionClient.LIVE)
            .setSmallIcon(android.R.drawable.stat_sys_download).setContentTitle("NDM is watching your PC")
            .setContentText("Get download and file sharing updates on this network.").setContentIntent(client.launcher(42672)).setOngoing(true).build();
        startForeground(42672,notification);
        client.watching=true;
        return START_NOT_STICKY;
    }
    @Override public void onTimeout(int startId,int fgsType){stopSelf();}
    @Override public void onDestroy(){CompanionClient.get(this).watching=false;super.onDestroy();}
    @Override public IBinder onBind(Intent intent){return null;}
}
