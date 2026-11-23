package io.github.yeloboi.ndm;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.*;
import android.net.Uri;
import android.os.*;
import android.util.Base64;
import android.view.accessibility.AccessibilityNodeInfo;
import androidx.core.content.FileProvider;
import org.json.JSONObject;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

public final class AndroidSmoke extends Instrumentation {
    private Bundle options;
    @Override public void onCreate(Bundle options){super.onCreate(options);this.options=options;start();}
    @Override public void onStart(){
        Bundle report=new Bundle();
        try{
            Context app=getTargetContext();
            MainActivity activity=(MainActivity)startActivitySync(new Intent(app,MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            awaitText("Downloads");
            String qr=new String(Base64.decode(options.getString("ndm_qr"),Base64.DEFAULT), StandardCharsets.UTF_8);
            JSONObject value=new JSONObject(qr.substring(5));
            require(!PcLink.isBluetoothQr(qr),"Network QR was mistaken for Bluetooth");
            PcLink link=PcLink.pair(app,qr,"be1c1a37-a1bd-48bb-8c0a-9d474edcbe65","Emulator",true);
            require(link.host.equals("127.0.0.1:42672"),"Pairing did not fall back to the reachable address");
            link.save(app);
            PcLink remembered=PcLink.load(app);
            require(remembered!=null&&remembered.token.equals(link.token),"Could not restore Android Keystore session");
            require(remembered.at(value.getString("host")).status().route().host.equals(link.host),"Reconnect did not fall back to the saved address");
            require(remembered.object("GET","/status",null).getJSONArray("downloads").length()==1,"Could not read remote downloads");
            JSONObject manifest=new JSONObject().put("tag","v0.1.0-alpha.10").put("android",new JSONObject()
                .put("code",10).put("name","NDM-0.1.0-alpha.10-android.apk").put("sha256","a".repeat(64)).put("size",40));
            require(AppUpdates.candidate(manifest,9).code()==10 && AppUpdates.candidate(manifest,10)==null,"Update version comparison failed");
            manifest.getJSONObject("android").put("name","https://example.com/unsafe.apk");
            try{AppUpdates.candidate(manifest,9);throw new AssertionError("Foreign APK was accepted");}
            catch(java.io.IOException expected){}
            try{
                value.put("pin","0000000000000000000000000000000000000000000000000000000000000000");
                PcLink.pair(app,"NDM1:"+value,"be1c1a37-a1bd-48bb-8c0a-9d474edcbe65","Untrusted",true);
                throw new AssertionError("Untrusted TLS identity was accepted");
            }catch(javax.net.ssl.SSLException expected){}
            File directory=new File(app.getFilesDir(),"received/smoke");
            require(directory.mkdirs()||directory.isDirectory(),"Could not prepare APK fixture");
            File copy=new File(directory,"app.apk");
            Files.copy(new File(app.getApplicationInfo().sourceDir).toPath(),copy.toPath(),java.nio.file.StandardCopyOption.REPLACE_EXISTING);
            Uri uri=FileProvider.getUriForFile(app,app.getPackageName()+".files",copy);
            ApkInstall.Details apk=ApkInstall.inspect(app,uri);
            require(app.getPackageName().equals(apk.packageName())&&apk.size()>0&&apk.icon()!=null,"APK metadata is incomplete");
            apk.file().delete();
            runOnMainSync(()->activity.onNewIntent(new Intent(Intent.ACTION_VIEW).setData(uri)));
            awaitText("PACKAGE INSTALLER");
            awaitText("Install");
            PcLink.forget(app);
            report.putString("stream","\nNDM_ANDROID_SMOKE_OK\n");
            finish(Activity.RESULT_OK,report);
        }catch(Throwable error){
            report.putString("stream","\nNDM_ANDROID_SMOKE_FAILED: "+android.util.Log.getStackTraceString(error)+"\n");
            finish(Activity.RESULT_CANCELED,report);
        }
    }
    private void awaitText(String expected)throws InterruptedException{
        for(int i=0;i<80;i++){
            var tree=getUiAutomation().getRootInActiveWindow();
            if(tree!=null&&found(tree,expected))return;
            Thread.sleep(250);
        }
        throw new AssertionError("Android screen did not show: "+expected);
    }
    private boolean found(AccessibilityNodeInfo view,String expected){
        if(expected.contentEquals(view.getText()==null?"":view.getText()))return true;
        for(int i=0;i<view.getChildCount();i++){
            var child=view.getChild(i);
            if(child!=null&&found(child,expected))return true;
        }
        return false;
    }
    private void require(boolean valid,String explanation){if(!valid)throw new AssertionError(explanation);}
}
