package io.github.yeloboi.ndm;

import android.content.Context;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.os.Build;
import org.json.JSONObject;
import javax.net.ssl.HttpsURLConnection;
import java.io.*;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HashSet;
import java.util.UUID;
import java.util.function.IntConsumer;

final class AppUpdates {
    static final String RELEASES="https://github.com/YeloBoi/next-download-manager/releases";
    private static final long MAX_APK=128L*1024*1024;

    record Update(String tag, long code, String name, long size, String sha256) {
        String notes(){return RELEASES+"/tag/"+tag;}
    }

    static Update candidate(JSONObject feed,long installed)throws Exception{
        String tag=feed.getString("tag");
        JSONObject apk=feed.getJSONObject("android");
        long code=apk.getLong("code");
        if(code<=installed)return null;
        String name=apk.getString("name"),hash=apk.getString("sha256");
        long size=apk.getLong("size");
        if(!tag.matches("v[0-9]+\\.[0-9]+\\.[0-9]+(?:-[0-9A-Za-z.-]+)?")
            ||!name.equals("NDM-"+tag.substring(1)+"-android.apk")||size<=0||size>MAX_APK||!hash.matches("[a-f0-9]{64}"))
            throw new IOException("Invalid NDM update details.");
        return new Update(tag,code,name,size,hash);
    }

    static Update check(Context context)throws Exception{
        HttpsURLConnection connection=open(RELEASES+"/latest/download/ndm-updates.json");
        try{
            if(connection.getResponseCode()==404)return null;
            try(InputStream input=stream(connection);ByteArrayOutputStream json=new ByteArrayOutputStream()){
                byte[] bytes=new byte[4096];int count;
                while((count=input.read(bytes))!=-1){
                    if(json.size()+count>64*1024)throw new IOException("NDM update details are too large.");
                    json.write(bytes,0,count);
                }
                long installed=code(context.getPackageManager().getPackageInfo(context.getPackageName(),0));
                return candidate(new JSONObject(new String(json.toByteArray(),StandardCharsets.UTF_8)),installed);
            }
        }finally{ connection.disconnect(); }
    }

    static ApkInstall.Details download(Context context,Update update,IntConsumer progress)throws Exception{
        File path=new File(context.getCacheDir(),"update-"+UUID.randomUUID()+".apk");
        HttpsURLConnection connection=open(RELEASES+"/download/"+update.tag()+"/"+update.name());
        try{
            MessageDigest hash=MessageDigest.getInstance("SHA-256");
            long total=0;int last=-1;
            try(InputStream input=stream(connection);OutputStream output=new FileOutputStream(path)){
                byte[] buffer=new byte[65536];int count;
                while((count=input.read(buffer))!=-1){
                    total+=count;
                    if(total>update.size())throw new IOException("The update download is larger than expected.");
                    hash.update(buffer,0,count);output.write(buffer,0,count);
                    int percent=(int)(total*100/update.size());
                    if(percent!=last){last=percent;progress.accept(percent);}
                }
            }
            if(total!=update.size()||!MessageDigest.isEqual(hash.digest(),hexBytes(update.sha256())))
                throw new IOException("The update download failed verification.");
            verifyPackage(context,path,update);
            return ApkInstall.details(context,path);
        }catch(Exception error){path.delete();throw error;}
        finally{connection.disconnect();}
    }

    private static void verifyPackage(Context context,File file,Update update)throws Exception{
        PackageManager manager=context.getPackageManager();
        int flags=Build.VERSION.SDK_INT>=28?PackageManager.GET_SIGNING_CERTIFICATES:PackageManager.GET_SIGNATURES;
        PackageInfo current=manager.getPackageInfo(context.getPackageName(),flags);
        PackageInfo next=manager.getPackageArchiveInfo(file.getAbsolutePath(),flags);
        if(next==null||!current.packageName.equals(next.packageName)||!update.tag().substring(1).equals(next.versionName)
            ||code(next)!=update.code()||code(next)<=code(current))
            throw new IOException("This file is not the expected newer NDM package.");
        Signature[] installed=signers(current), incoming=signers(next);
        if(installed==null||installed.length==0||incoming==null||
            !new HashSet<>(Arrays.asList(installed)).equals(new HashSet<>(Arrays.asList(incoming))))
            throw new IOException("The update is not signed with the installed NDM key.");
    }

    @SuppressWarnings("deprecation")
    private static Signature[] signers(PackageInfo info){
        return Build.VERSION.SDK_INT>=28?(info.signingInfo==null?null:info.signingInfo.getApkContentsSigners()):info.signatures;
    }

    @SuppressWarnings("deprecation")
    private static long code(PackageInfo info){return Build.VERSION.SDK_INT>=28?info.getLongVersionCode():info.versionCode;}

    private static byte[] hexBytes(String hex){
        byte[] bytes=new byte[hex.length()/2];
        for(int i=0;i<bytes.length;i++)bytes[i]=(byte)Integer.parseInt(hex.substring(i*2,i*2+2),16);
        return bytes;
    }

    private static HttpsURLConnection open(String address)throws IOException{
        URL url=new URL(address);
        for(int n=0;n<4;n++){
            if(!"https".equals(url.getProtocol())||!Arrays.asList("github.com","release-assets.githubusercontent.com",
                "objects.githubusercontent.com").contains(url.getHost())||url.getUserInfo()!=null)
                throw new IOException("The update redirected outside GitHub.");
            HttpsURLConnection connection=(HttpsURLConnection)url.openConnection();
            connection.setInstanceFollowRedirects(false);connection.setConnectTimeout(6000);connection.setReadTimeout(15000);
            boolean keep=false;
            try{
                int status=connection.getResponseCode();
                if(status!=301&&status!=302&&status!=303&&status!=307&&status!=308){keep=true;return connection;}
                String redirect=connection.getHeaderField("Location");
                if(redirect==null)throw new IOException("Invalid GitHub update redirect.");
                url=new URL(url,redirect);
            }finally{if(!keep)connection.disconnect();}
        }
        throw new IOException("Too many GitHub update redirects.");
    }

    private static InputStream stream(HttpsURLConnection connection)throws IOException{
        if(connection.getResponseCode()!=200)throw new IOException("GitHub returned HTTP "+connection.getResponseCode()+".");
        return connection.getInputStream();
    }
}
