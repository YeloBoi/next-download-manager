package io.github.yeloboi.ndm;

import android.app.DownloadManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.SharedPreferences;
import android.database.Cursor;
import android.net.Uri;
import android.os.Environment;
import org.json.*;
import java.io.*;
import java.net.*;
import java.util.*;

final class PhoneDownloads {
    final DownloadManager manager;
    final SharedPreferences prefs;
    private final Context context;
    private final JSONObject jobs;
    private final JSONObject usage;

    PhoneDownloads(Context context) {
        this.context=context;
        manager=context.getSystemService(DownloadManager.class);
        prefs=context.getSharedPreferences("phone_downloads",Context.MODE_PRIVATE);
        jobs=stored("jobs");usage=stored("usage");
    }

    private JSONObject stored(String key){try{return new JSONObject(prefs.getString(key,"{}"));}catch(JSONException error){return new JSONObject();}}
    static URI link(String raw) throws Exception {
        URI uri=new URI(raw.trim());
        if (raw.length()>16384 || uri.getHost()==null || uri.getRawUserInfo()!=null ||
            !("https".equalsIgnoreCase(uri.getScheme())||"http".equalsIgnoreCase(uri.getScheme())))
            throw new IOException("Enter a direct HTTP or HTTPS link.");
        return uri;
    }
    static String safe(String name) {
        String clean=name.replaceAll("[\\p{Cntrl}\\u202a-\\u202e\\u2066-\\u2069<>:\"/\\\\|?*]","_").trim().replaceAll("^\\.+|\\.+$","");
        if(clean.length()>150)clean=clean.substring(0,150);
        return clean.trim().isEmpty()?"download":clean;
    }
    static String nameFrom(URI uri) {
        String path=uri.getPath();
        return safe(path==null?"download":path.substring(path.lastIndexOf('/')+1));
    }
    static String category(String file) {
        String ext=file.substring(file.lastIndexOf('.')+1).toLowerCase(Locale.ROOT);
        return switch(ext){
            case "mp4","mkv","webm","avi","mov","m4v" -> "video";
            case "mp3","wav","ogg","m4a","flac","aac" -> "audio";
            case "jpg","jpeg","png","gif","webp","heic","svg" -> "images";
            case "apk","exe","msi","msix","dmg","deb","rpm","appimage" -> "apps";
            case "iso","rom","xci","nsp","gba","nds" -> "games";
            case "zip","rar","7z","tar","gz","xz" -> "archives";
            case "pdf","doc","docx","txt","xls","xlsx","ppt","pptx","epub" -> "documents";
            default -> "other";
        };
    }

    record Preview(String name,long size,String resume){}
    static Preview preview(String raw) throws Exception {
        URI uri=link(raw);
        String name=nameFrom(uri);
        HttpURLConnection conn=null;
        try {
            conn=(HttpURLConnection)uri.toURL().openConnection();
            conn.setRequestMethod("HEAD");conn.setConnectTimeout(4000);conn.setReadTimeout(4000);
            conn.setRequestProperty("Accept-Encoding","identity");
            int status=conn.getResponseCode();
            if(status<200||status>=300)return new Preview(name,-1,"Unknown");
            String mime=conn.getContentType();
            if(mime!=null&&mime.startsWith("text/html"))throw new IllegalArgumentException("This link points to a web page. Choose a direct file link.");
            String disposition=conn.getHeaderField("Content-Disposition");
            if(disposition!=null){
                java.util.regex.Matcher utf8=java.util.regex.Pattern.compile("(?i)(?:^|;)\\s*filename\\*=utf-8''([^;]+)").matcher(disposition);
                java.util.regex.Matcher simple=java.util.regex.Pattern.compile("(?i)(?:^|;)\\s*filename=\"?([^;\"]+)").matcher(disposition);
                if(utf8.find())name=safe(URLDecoder.decode(utf8.group(1).replace("+","%2B"),"UTF-8"));
                else if(simple.find())name=safe(simple.group(1));
            }
            String range=conn.getHeaderField("Accept-Ranges");
            return new Preview(name,conn.getContentLengthLong(),range==null?"Unknown":range.equalsIgnoreCase("bytes")?"Available":"Unavailable");
        }catch(IOException ignored){return new Preview(name,-1,"Unknown");}
        finally{if(conn!=null)conn.disconnect();}
    }

    synchronized void enqueue(String input,String name) throws Exception {
        URI url=link(input);
        name=safe(name);
        DownloadManager.Request req=new DownloadManager.Request(Uri.parse(url.toString()))
            .setTitle(name).setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
            .setDestinationInExternalPublicDir(Environment.DIRECTORY_DOWNLOADS,"NDM/"+UUID.randomUUID()+"/"+name);
        long id=manager.enqueue(req);
        jobs.put(Long.toString(id),new JSONObject().put("name",name).put("counted",false));
        save();
    }

    synchronized List<Local> inspect() {
        List<Local> list=new ArrayList<>();
        boolean fresh=false;
        Iterator<String> keys=jobs.keys();
        while(keys.hasNext()){
            String key=keys.next();long id=Long.parseLong(key);
            JSONObject entry=jobs.optJSONObject(key);
            if(entry==null)continue;
            try(Cursor row=manager.query(new DownloadManager.Query().setFilterById(id))){
                if(row==null||!row.moveToFirst())continue;
                int status=row.getInt(row.getColumnIndexOrThrow(DownloadManager.COLUMN_STATUS));
                int reason=row.getInt(row.getColumnIndexOrThrow(DownloadManager.COLUMN_REASON));
                long bytes=row.getLong(row.getColumnIndexOrThrow(DownloadManager.COLUMN_BYTES_DOWNLOADED_SO_FAR));
                long total=row.getLong(row.getColumnIndexOrThrow(DownloadManager.COLUMN_TOTAL_SIZE_BYTES));
                String name=entry.getString("name");
                Uri uri=status==DownloadManager.STATUS_SUCCESSFUL?manager.getUriForDownloadedFile(id):null;
                boolean completed=false;
                if(status==DownloadManager.STATUS_SUCCESSFUL&&!entry.optBoolean("counted")){
                    long time=row.getLong(row.getColumnIndexOrThrow(DownloadManager.COLUMN_LAST_MODIFIED_TIMESTAMP));
                    long hour=(time>0?time:System.currentTimeMillis())/3600000;
                    String bucket=hour+":"+category(name);
                    usage.put(bucket,usage.optLong(bucket,0)+Math.max(bytes,0));
                    entry.put("counted",true);fresh=true;completed=true;
                }
                list.add(new Local(id,name,status,reason,bytes,total,uri,completed));
            }catch(Exception ignored){}
        }
        if(fresh)save();
        list.sort(Comparator.comparingLong(Local::id).reversed());
        return list;
    }
    synchronized void remove(long id){manager.remove(id);jobs.remove(Long.toString(id));save();}
    synchronized JSONArray usage() throws JSONException {
        JSONArray list=new JSONArray();long min=System.currentTimeMillis()/3600000-31*24;
        Iterator<String> keys=usage.keys();boolean changed=false;
        while(keys.hasNext()){
            String key=keys.next();String[] split=key.split(":");long hour=Long.parseLong(split[0]);
            if(hour>=min)list.put(new JSONObject().put("hour",hour).put("category",split[1]).put("bytes",usage.getLong(key)).put("device","mobile"));
            else { keys.remove();changed=true; }
        }
        if(changed)save();
        return list;
    }
    private void save(){prefs.edit().putString("jobs",jobs.toString()).putString("usage",usage.toString()).commit();}

    record Local(long id,String name,int status,int reason,long bytes,long total,Uri uri,boolean justCompleted) {
        boolean apk(){return name.toLowerCase(Locale.ROOT).endsWith(".apk");}
    }

    static void copyUri(ContentResolver resolver, Uri uri, File target,long max) throws IOException {
        try(InputStream in=resolver.openInputStream(uri);FileOutputStream out=new FileOutputStream(target)){
            if(in==null)throw new IOException("Cannot open this file.");
            byte[] buffer=new byte[65536];long total=0;int n;
            while((n=in.read(buffer))!=-1){
                total+=n;if(total>max)throw new IOException("The file exceeds the size limit.");
                out.write(buffer,0,n);
            }
            out.getFD().sync();
        }
    }
}
