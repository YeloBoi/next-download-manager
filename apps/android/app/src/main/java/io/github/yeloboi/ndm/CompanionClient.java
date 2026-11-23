package io.github.yeloboi.ndm;

import android.app.*;
import android.content.*;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import org.json.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;

final class CompanionClient {
    private static CompanionClient instance;
    static synchronized CompanionClient get(Context context) {
        if(instance==null)instance=new CompanionClient(context.getApplicationContext());
        return instance;
    }
    static final String LIVE="ndm_connection", EVENTS="ndm_events";
    final Context context;
    final SharedPreferences prefs;
    final PhoneDownloads local;
    final ExecutorService commands=Executors.newFixedThreadPool(2);
    private final ScheduledExecutorService poller=Executors.newSingleThreadScheduledExecutor();
    private final ScheduledExecutorService mover=Executors.newSingleThreadScheduledExecutor();
    final String id;
    volatile PcLink pc;
    volatile boolean foreground=false, watching=false, online=false;
    volatile String issue="";
    volatile String transferIssue="";
    volatile JSONArray downloads=new JSONArray(), transfers=new JSONArray(), usage=new JSONArray();
    volatile List<PhoneDownloads.Local> phoneJobs=Collections.emptyList();
    private final Set<String> canceled=ConcurrentHashMap.newKeySet();
    private final Set<String> selected=ConcurrentHashMap.newKeySet();
    private volatile long syncAt=0;

    private CompanionClient(Context context){
        this.context=context;
        prefs=context.getSharedPreferences("ndm",Context.MODE_PRIVATE);
        String saved=prefs.getString("id","");
        id=saved.isEmpty()?UUID.randomUUID().toString():saved;
        if(saved.isEmpty())prefs.edit().putString("id",id).commit();
        local=new PhoneDownloads(context);
        pc=PcLink.load(context);
        canceled.addAll(prefs.getStringSet("canceled",Collections.emptySet()));
        selected.addAll(prefs.getStringSet("selected",Collections.emptySet()));
        createChannels();
        poller.scheduleWithFixedDelay(this::poll,0,1200,TimeUnit.MILLISECONDS);
        mover.scheduleWithFixedDelay(this::pump,0,100,TimeUnit.MILLISECONDS);
    }

    private void createChannels(){
        NotificationManager nm=context.getSystemService(NotificationManager.class);
        nm.createNotificationChannel(new NotificationChannel(LIVE,"PC connection",NotificationManager.IMPORTANCE_LOW));
        nm.createNotificationChannel(new NotificationChannel(EVENTS,"Downloads and file sharing",NotificationManager.IMPORTANCE_DEFAULT));
    }
    PendingIntent launcher(int id) {return PendingIntent.getActivity(context,id,new Intent(context,MainActivity.class),PendingIntent.FLAG_IMMUTABLE|PendingIntent.FLAG_UPDATE_CURRENT);}
    void notifyEvent(String name,String body,int id){
        Notification n=new Notification.Builder(context,EVENTS).setSmallIcon(android.R.drawable.stat_sys_download_done)
            .setContentTitle(name).setContentText(body).setAutoCancel(true).setContentIntent(launcher(id)).build();
        try{context.getSystemService(NotificationManager.class).notify(id,n);}catch(SecurityException ignored){}
    }
    private synchronized void saveChoices(){
        prefs.edit().putStringSet("selected",new HashSet<>(selected)).putStringSet("canceled",new HashSet<>(canceled)).commit();
    }
    void pair(String qr,String label,boolean remember) throws Exception {
        PcLink paired=PcLink.pair(qr,id,label.length()>64?label.substring(0,64):label,remember);
        if(remember)paired.save(context);else PcLink.forget(context);
        PcLink prior=pc;
        if(prior==null||!prior.pin.equals(paired.pin))
            prefs.edit().remove("pendingPC").remove("completedPC").remove("pcBaseline").remove("seenOffers").commit();
        syncAt=0;issue="";transferIssue="";pc=paired;
    }
    void disconnect() {
        PcLink before=pc;pc=null;online=false;downloads=new JSONArray();transfers=new JSONArray();usage=new JSONArray();
        PcLink.forget(context);
        if(before!=null)try{before.object("DELETE","/device",null);}catch(Exception ignored){}
    }
    private PcLink needPc() throws IOException {PcLink value=pc;if(value==null)throw new IOException("Pair your PC first.");return value;}
    void control(String type,String id,List<String> urls) throws Exception {
        JSONObject data=new JSONObject().put("type",type);
        if(id!=null)data.put("id",id);
        if(urls!=null){JSONArray array=new JSONArray();for(String url:urls)array.put(PhoneDownloads.link(url).toString());data.put("urls",array);}
        needPc().object("POST","/download",data);
    }

    private void poll() {
        if(!foreground&&!watching)return;
        phoneJobs=local.inspect();
        PcLink link=pc;if(link==null)return;
        try{
            JSONObject state=link.object("GET","/status",null);
            if(pc!=link)return;
            JSONArray jobs=state.getJSONArray("downloads");
            JSONArray shares=state.getJSONArray("transfers");
            Set<String> pending=new HashSet<>();
            Set<String> old=prefs.getStringSet("pendingPC",Collections.emptySet());
            Set<String> oldCompleted=prefs.getStringSet("completedPC",Collections.emptySet());
            Set<String> completed=new HashSet<>();
            boolean initialized=prefs.getBoolean("pcBaseline",false);
            for(int i=0;i<jobs.length();i++){
                JSONObject j=jobs.getJSONObject(i);String key=j.getString("id");
                if(j.getString("status").equals("completed")){
                    completed.add(key);
                    if(old.contains(key)||(initialized&&!oldCompleted.contains(key)))notifyEvent("PC download complete",j.optString("name"),key.hashCode());
                } else pending.add(key);
            }
            Set<String> seen=new HashSet<>(prefs.getStringSet("seenOffers",Collections.emptySet()));
            Set<String> liveOffers=new HashSet<>();
            for(int i=0;i<shares.length();i++){
                JSONObject j=shares.getJSONObject(i);String key=j.getString("id");
                liveOffers.add(key);
                if(!j.optBoolean("to_pc")&&j.optString("phase").equals("ready")&&seen.add(key))
                    notifyEvent("File offered by PC",j.optString("name"),key.hashCode());
            }
            seen.retainAll(liveOffers);
            prefs.edit().putStringSet("pendingPC",pending).putStringSet("completedPC",completed).putBoolean("pcBaseline",true).putStringSet("seenOffers",seen).apply();
            downloads=jobs;transfers=shares;online=true;issue="";
            if(System.currentTimeMillis()>syncAt){
                JSONArray tally=local.usage();
                byte[] response=link.request("POST","/usage",tally.toString().getBytes(StandardCharsets.UTF_8),"application/json",3*1024*1024);
                usage=new JSONArray(new String(response,StandardCharsets.UTF_8));
                syncAt=System.currentTimeMillis()+15000;
            }
        }catch(Exception error){
            if(pc!=link)return;
            if(error instanceof PcLink.HttpFailure&&((PcLink.HttpFailure)error).code==401){
                PcLink.forget(context);pc=null;downloads=new JSONArray();transfers=new JSONArray();usage=new JSONArray();issue="Access ended. Pair again on the PC.";
            } else issue="PC unavailable. Check that both devices are on the same network and NDM is open.";
            online=false;
        }
    }

    private File dir(String name)throws IOException{
        File folder=new File(context.getFilesDir(),name);
        if(!folder.isDirectory()&&!folder.mkdirs())throw new IOException("Could not create the share folder.");
        return folder;
    }
    private static boolean validId(String id){try{return UUID.fromString(id).toString().equals(id);}catch(Exception e){return false;}}
    private File staged(String id) throws IOException {if(!validId(id))throw new IOException("Invalid transfer ID.");return new File(dir("outgoing"),id+".part");}
    File received(JSONObject offer) throws Exception {
        String id=offer.getString("id");if(!validId(id))throw new IOException("Invalid transfer ID.");
        return new File(dir("received/"+id),PhoneDownloads.safe(offer.getString("name")));
    }
    static String name(Context context,Uri uri){
        try(Cursor c=context.getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null)){
            if(c!=null&&c.moveToFirst()&&!c.isNull(0))return PhoneDownloads.safe(c.getString(0));
        }catch(Exception ignored){}
        return "shared-file";
    }
    private File uploadMeta(String id)throws IOException {return new File(dir("outgoing"),id+".json");}
    void stage(List<Uri> uris)throws Exception{
        PcLink target=needPc();
        if(uris.isEmpty()||uris.size()>32)throw new IOException("Choose between 1 and 32 files.");
        for(Uri uri:uris){
            if(!"content".equals(uri.getScheme()))throw new IOException("Select a file from the Android picker.");
            String id=UUID.randomUUID().toString();
            File path=staged(id);
            File meta=uploadMeta(id);
            File pending=new File(meta.getPath()+".tmp");
            try{
                PhoneDownloads.copyUri(context.getContentResolver(),uri,path,128L*1024*1024*1024);
                JSONObject offer=new JSONObject().put("id",id).put("total",path.length()).put("name",name(context,uri)).put("pin",target.pin);
                try(FileOutputStream out=new FileOutputStream(pending)){
                    out.write(offer.toString().getBytes(StandardCharsets.UTF_8));out.getFD().sync();
                }
                if(!pending.renameTo(meta))throw new IOException("Could not prepare the file for transfer.");
            }catch(Exception error){path.delete();pending.delete();throw error;}
        }
    }
    List<JSONObject> stagedOffers(){
        ArrayList<JSONObject> offers=new ArrayList<>();
        try {
            File[] files=dir("outgoing").listFiles((dir,name)->name.endsWith(".json"));
            if(files!=null)for(File file:files)try(InputStream in=new FileInputStream(file)){
                byte[] contents=new byte[(int)Math.min(file.length(),4096)];
                if(in.read(contents)==contents.length)offers.add(new JSONObject(new String(contents,StandardCharsets.UTF_8)));
            }catch(Exception ignored){}
        }catch(IOException ignored){}
        return offers;
    }
    boolean chosen(String id){return selected.contains(id);}
    boolean paused(String id){return canceled.contains(id);}
    void pause(String id,boolean retry)throws Exception{
        if(!validId(id))throw new IOException("Invalid transfer ID.");
        if(retry)canceled.remove(id);else canceled.add(id);
        transferIssue="";
        saveChoices();
        PcLink dst=pc;
        if(dst!=null)try{dst.object(retry?"POST":"DELETE","/transfer/"+id,null);}catch(PcLink.HttpFailure e){if(e.code!=404)throw e;}
    }
    void accept(String id)throws Exception{
        if(!validId(id))throw new IOException("Invalid transfer ID.");
        selected.add(id);pause(id,true);saveChoices();
    }

    private void pump(){
        PcLink link=pc;
        if((!foreground&&!watching)||link==null||!online)return;
        try{
            for(JSONObject upload:stagedOffers()){
                if(pc!=link||!online)return;
                if(!link.pin.equals(upload.optString("pin")))continue;
                upload.remove("pin");
                String id=upload.getString("id");
                if(canceled.contains(id))continue;
                JSONObject p=link.object("POST","/transfer",upload);
                if(p.optString("phase").equals("ready")){
                    long pos=p.optLong("bytes"),total=p.getLong("total");
                    File file=staged(id);
                    if(!file.isFile()||file.length()!=total)throw new IOException("Staged file is no longer available.");
                    try(RandomAccessFile source=new RandomAccessFile(file,"r")){
                        for(int chunk=0;chunk<12&&!canceled.contains(id)&&pc==link;chunk++){
                            byte[] data=new byte[(int)Math.min(PcLink.CHUNK,total-pos)];
                            source.seek(pos);source.readFully(data);
                            byte[] ack=link.request("POST","/transfer/"+id+"/chunk?offset="+pos,data,"application/octet-stream",8192);
                            p=new JSONObject(new String(ack,StandardCharsets.UTF_8));
                            long confirmed=p.getLong("bytes");
                            if(confirmed<0||confirmed>total)throw new IOException("Invalid transfer offset received.");
                            pos=confirmed;
                            if(!p.optString("phase").equals("ready"))break;
                        }
                    }
                }
                if(p.optString("phase").equals("complete")){
                    uploadMeta(id).delete();staged(id).delete();
                    notifyEvent("File sent to PC",p.optString("name"),id.hashCode());
                }
            }
            JSONArray offers=transfers;
            for(int i=0;i<offers.length();i++){
                JSONObject offer=offers.getJSONObject(i);
                String id=offer.getString("id");
                if(pc!=link||!online)return;
                if(offer.optBoolean("to_pc")||!selected.contains(id)||canceled.contains(id)||!offer.optString("phase").equals("ready"))continue;
                receive(link,offer,id);
            }
            transferIssue="";
        }catch(Exception error){ transferIssue=error.getMessage()==null?"Transfer interrupted; NDM can resume it.":error.getMessage(); }
    }

    private void receive(PcLink link,JSONObject offer,String id)throws Exception{
        File finalFile=received(offer);
        File partial=new File(finalFile.getParentFile(),".partial");
        long total=offer.getLong("total");
        long pos=0;
        if(!finalFile.isFile()){
            try(RandomAccessFile output=new RandomAccessFile(partial,"rw")){
                pos=output.length();
                if(pos>total)throw new IOException("Invalid saved offset for "+offer.getString("name"));
                for(int chunk=0;chunk<12&&pos<total&&!canceled.contains(id)&&pc==link;chunk++){
                    byte[] data=link.request("GET","/transfer/"+id+"/chunk?offset="+pos,null,"application/octet-stream",PcLink.CHUNK);
                    if(data.length==0||data.length>total-pos)throw new IOException("Invalid transfer chunk.");
                    output.seek(pos);output.write(data);pos+=data.length;
                }
                output.getFD().sync();
            }
            if(pos==total&&!canceled.contains(id)&&!partial.renameTo(finalFile))throw new IOException("Could not finish saving the file.");
        }else { pos=finalFile.length();if(pos!=total)throw new IOException("Received file size no longer matches."); }
        if(pos<total||finalFile.isFile()){
            link.object("POST","/transfer/"+id+"/ack",new JSONObject().put("offset",pos));
            if(finalFile.isFile()&&selected.remove(id)){
                saveChoices();notifyEvent("File received from PC",offer.optString("name"),id.hashCode());
            }
        }
    }
}
