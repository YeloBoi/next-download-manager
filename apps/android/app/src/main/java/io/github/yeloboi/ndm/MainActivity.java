package io.github.yeloboi.ndm;

import android.Manifest;
import android.annotation.SuppressLint;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothManager;
import android.app.*;
import android.content.*;
import android.content.pm.PackageInstaller;
import android.content.pm.PackageManager;
import android.content.res.Configuration;
import android.graphics.*;
import android.graphics.drawable.ColorDrawable;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.*;
import android.provider.Settings;
import android.text.InputType;
import android.view.*;
import android.widget.*;
import androidx.core.content.FileProvider;
import com.google.zxing.integration.android.IntentIntegrator;
import org.json.*;
import java.io.File;
import java.time.*;
import java.time.format.DateTimeFormatter;
import java.util.*;

public final class MainActivity extends Activity {
    private static final int PICK_FILES=201,PICK_APK=202,INSTALL_PERMISSION=203;
    private static final int BLUETOOTH_PERMISSION=204,ENABLE_BLUETOOTH=205,SCAN_BLUETOOTH=206,SCAN_NETWORK=207;
    private final Handler main=new Handler(Looper.getMainLooper());
    private CompanionClient client;
    private LinearLayout board,tabs;
    private TextView connection;
    private ScrollView scroll;
    private int active=0,period=24;
    private String category="", signature="";
    private boolean dark;
    private int surface,bg,card,ink,sub,accent,actionInk,line;
    private ApkInstall.Details install;
    private Dialog apkDialog;
    private ProgressBar apkProgress;
    private TextView apkStatus;
    private Button installButton;
    private boolean installing=false,waitingForPermission=false;
    private boolean inspectBusy=false;
    private final Set<Long> seenLocal=new HashSet<>();
    private final Map<Long,long[]> localRates=new HashMap<>();
    private Intent pendingShare;
    private LinearLayout updateBanner;
    private ProgressBar updateBar;
    private AppUpdates.Update available;
    private long nextUpdateCheck;
    private boolean checkingUpdate,downloadingUpdate;

    @Override public void onCreate(Bundle state){
        super.onCreate(state);
        client=CompanionClient.get(this);
        dark=(getResources().getConfiguration().uiMode&Configuration.UI_MODE_NIGHT_MASK)!=Configuration.UI_MODE_NIGHT_NO;
        bg=color(dark?"#12151C":"#F0F3F9");surface=color(dark?"#191E29":"#FFFFFF");card=color(dark?"#232A38":"#E9EEF8");
        ink=color(dark?"#F3F6FF":"#151B2C");sub=color(dark?"#A9B5CC":"#4F607B");accent=color(dark?"#8CB6FF":"#1358BF");
        actionInk=color(dark?"#102342":"#FFFFFF");line=color(dark?"#33405A":"#C5D0E5");
        getWindow().setStatusBarColor(bg);getWindow().setNavigationBarColor(bg);
        if(!dark)getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR|View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
        build();
        handleIntent(getIntent());
    }
    @Override protected void onResume(){super.onResume();client.foreground=true;main.removeCallbacks(tick);main.post(tick);checkUpdate(false);}
    @Override protected void onPause(){client.foreground=false;main.removeCallbacks(tick);super.onPause();}
    @Override protected void onNewIntent(Intent intent){super.onNewIntent(intent);setIntent(intent);handleIntent(intent);}

    private final Runnable tick=new Runnable(){
        @Override public void run(){
            installFeedback();
            if(!inspectBusy&&!installing&&!downloadingUpdate&&apkDialog==null){
                for(PhoneDownloads.Local job:client.phoneJobs)
                    if(job.justCompleted()&&job.apk()&&job.uri()!=null&&seenLocal.add(job.id())){inspectApk(job.uri());break;}
            }
            render(false);main.postDelayed(this,850);
        }
    };

    private int color(String hex){return Color.parseColor(hex);}
    private int dp(float value){return Math.round(value*getResources().getDisplayMetrics().density);}
    private GradientDrawable shape(int color,int radius){GradientDrawable d=new GradientDrawable();d.setColor(color);d.setCornerRadius(dp(radius));return d;}
    private LinearLayout column(){LinearLayout l=new LinearLayout(this);l.setOrientation(LinearLayout.VERTICAL);return l;}
    private TextView text(String value,int size,int color){TextView v=new TextView(this);v.setText(value);v.setTextSize(size);v.setTextColor(color);v.setLineSpacing(dp(2),1);return v;}
    private TextView strong(String value,int size){TextView t=text(value,size,ink);t.setTypeface(android.graphics.Typeface.DEFAULT,android.graphics.Typeface.BOLD);return t;}
    private void gap(LinearLayout l,int amount){l.addView(new Space(this),new LinearLayout.LayoutParams(1,dp(amount)));}
    private void item(LinearLayout list,View view){LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,-2);lp.bottomMargin=dp(10);list.addView(view,lp);}
    private LinearLayout panel(){LinearLayout c=column();c.setPadding(dp(16),dp(14),dp(16),dp(14));c.setBackground(shape(surface,20));return c;}
    private Button button(String label,boolean primary,Runnable tap){
        Button b=new Button(this);b.setAllCaps(false);b.setText(label);b.setTextSize(13);b.setTextColor(primary?actionInk:ink);
        b.setMinHeight(dp(46));b.setMinimumHeight(dp(46));b.setMinimumWidth(0);b.setMinWidth(0);b.setPadding(dp(13),dp(6),dp(13),dp(6));
        b.setBackground(new android.graphics.drawable.RippleDrawable(android.content.res.ColorStateList.valueOf(color("#40718FC0")),shape(primary?accent:card,15),null));
        b.setOnClickListener(v->tap.run());return b;
    }
    private void actions(LinearLayout parent,Button... choices){
        if(choices.length==0)return;
        LinearLayout row=new LinearLayout(this);row.setGravity(Gravity.CENTER_VERTICAL);
        for(Button b:choices){LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(0,-2,1);lp.setMarginEnd(dp(6));row.addView(b,lp);}
        gap(parent,12);parent.addView(row);
    }
    private ProgressBar progress(long bytes,long total){
        ProgressBar p=new ProgressBar(this,null,android.R.attr.progressBarStyleHorizontal);
        p.setIndeterminate(total<0);p.setMax(1000);if(total>=0)p.setProgress(total==0?0:(int)Math.min(1000,bytes*1000/Math.max(1,total)));
        p.setProgressTintList(android.content.res.ColorStateList.valueOf(accent));
        p.setIndeterminateTintList(android.content.res.ColorStateList.valueOf(accent));
        p.setContentDescription(total>0?Math.min(100,bytes*100/total)+" percent":"Progress");
        return p;
    }
    private void build(){
        LinearLayout root=column();root.setBackgroundColor(bg);
        if(Build.VERSION.SDK_INT>=35)root.setOnApplyWindowInsetsListener((view,insets)->{
            android.graphics.Insets safe=insets.getInsets(WindowInsets.Type.systemBars()|WindowInsets.Type.displayCutout());
            view.setPadding(safe.left,safe.top,safe.right,safe.bottom);
            return insets;
        });
        LinearLayout head=new LinearLayout(this);head.setPadding(dp(19),dp(12),dp(19),dp(10));head.setGravity(Gravity.CENTER_VERTICAL);
        ImageView icon=new ImageView(this);icon.setImageResource(R.drawable.ndm_mark);head.addView(icon,new LinearLayout.LayoutParams(dp(44),dp(44)));
        LinearLayout title=column();title.setPadding(dp(10),0,0,0);title.addView(strong("NDM",23));
        connection=text("Phone & PC downloads",12,sub);title.addView(connection);head.addView(title,new LinearLayout.LayoutParams(0,-2,1));
        root.addView(head);
        tabs=new LinearLayout(this);tabs.setPadding(dp(13),0,dp(13),dp(6));root.addView(tabs);
        scroll=new ScrollView(this);scroll.setClipToPadding(false);scroll.setFillViewport(false);
        board=column();board.setPadding(dp(16),dp(7),dp(16),dp(22));scroll.addView(board);
        root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        updateBanner=column();updateBanner.setPadding(dp(16),dp(9),dp(16),dp(10));updateBanner.setBackgroundColor(surface);
        root.addView(updateBanner,new LinearLayout.LayoutParams(-1,-2));paintUpdate();
        setContentView(root);root.requestApplyInsets();select(0);
    }
    private void select(int tab){
        active=tab;tabs.removeAllViews();String[] labels={"Phone","PC","Files","Usage"};
        for(int i=0;i<labels.length;i++){
            final int index=i;Button b=button(labels[i],tab==i,()->select(index));b.setSelected(tab==i);
            LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(0,-2,1);lp.setMarginEnd(dp(4));tabs.addView(b,lp);
        }
        render(true);scroll.scrollTo(0,0);
    }
    private void header(String title,String hint){
        item(board,strong(title,27));if(!hint.isEmpty()){item(board,text(hint,13,sub));}
    }
    private void empty(String title,String body){
        LinearLayout p=panel();p.addView(strong(title,16));gap(p,6);p.addView(text(body,13,sub));item(board,p);
    }
    private void render(boolean force){
        if(board==null)return;
        connection.setText(client.online?"PC connected · encrypted":client.pc!=null?"PC reconnecting":"Phone & PC downloads");
        String state=active+"|"+client.online+"|"+client.watching+"|"+client.issue+"|"+(client.pc==null?"":client.pc.host);
        state+=switch(active){
            case 0 -> client.phoneJobs.toString();
            case 1 -> client.downloads.toString();
            case 2 -> client.transfers.toString()+client.transferIssue+client.stagedOffers().toString()+client.prefs.getStringSet("selected",Collections.emptySet())+client.prefs.getStringSet("canceled",Collections.emptySet());
            default -> client.usage.toString()+client.phoneJobs.toString()+period+category;
        };
        if(!force&&signature.equals(state))return;
        signature=state;
        board.removeAllViews();
        switch(active){case 0 -> phone();case 1 -> pc();case 2 -> files();default -> usage();}
    }

    private void phone(){
        header("Downloads","Save links to this phone and preview APK details before installing.");
        LinearLayout tools=panel();tools.addView(strong("On this device",16));
        actions(tools,button("Add link",true,()->addLink(false)),button("Choose APK",false,()->pick(PICK_APK)));
        actions(tools,button("Check for NDM updates",false,()->checkUpdate(true)));
        item(board,tools);
        if(client.phoneJobs.isEmpty())empty("Nothing downloading yet","Add a direct link. Files are saved in Downloads → NDM.");
        long now=SystemClock.elapsedRealtime();
        for(PhoneDownloads.Local j:client.phoneJobs){
            LinearLayout p=panel();p.addView(strong(j.name(),16));
            String status=switch(j.status()){
                case DownloadManager.STATUS_SUCCESSFUL -> "Complete";
                case DownloadManager.STATUS_RUNNING -> "Downloading";
                case DownloadManager.STATUS_FAILED -> "Failed (code "+j.reason()+")";
                case DownloadManager.STATUS_PAUSED -> "Waiting for connection";
                default -> "Queued";
            };
            long[] prior=localRates.get(j.id());long speed=prior==null||now==prior[0]?0:Math.max(0,j.bytes()-prior[1])*1000/(now-prior[0]);
            if(prior==null||now-prior[0]>750)localRates.put(j.id(),new long[]{now,j.bytes()});
            p.addView(text(status+" · "+meter(j.bytes(),j.total(),speed),12,sub));gap(p,8);p.addView(progress(j.bytes(),j.total()));
            if(j.uri()!=null)actions(p,button(j.apk()?"Review APK":"Open",true,()->{if(j.apk())inspectApk(j.uri());else open(j.uri(),j.name());}));
            else actions(p,button("Remove",false,()->ask("Remove this download?","This also discards any partial file.","Remove",()->run(()->client.local.remove(j.id())))));
            item(board,p);
        }
    }

    private void pc(){
        header("Your PC","Connect over Bluetooth, local Wi-Fi, or the internet.");
        LinearLayout p=panel();p.addView(strong(client.online?"Connected":client.pc!=null?"Reconnecting":"Pair a PC",16));gap(p,5);
        p.addView(text(client.pc!=null?(client.online?client.pc.routeLabel():client.issue.isEmpty()?"Connecting to "+client.pc.routeLabel():client.issue):
            client.issue.isEmpty()?"In NDM for Windows, open Settings → Your phone, then show the QR code.":client.issue,13,sub));
        if(client.pc==null)actions(p,button("Scan QR code",true,this::scan));
        else {
            actions(p,button("Add to PC",true,()->addLink(true)),button("Forget PC",false,()->ask("Forget this PC?","You can pair it again from Windows Settings.","Forget",()->run(()->{client.disconnect();main.post(()->stopService(new Intent(this,CompanionService.class)));}))));
            actions(p,button(client.watching?"Stop watching":"Watch in background",false,this::toggleWatch),button("Scan again",false,this::scan));
        }
        item(board,p);
        if(!client.online)return;
        LinearLayout queue=panel();queue.addView(strong("PC queue",15));
        actions(queue,button("Start queue",false,()->control("start_queue",null)),button("Pause queue",false,()->control("pause_queue",null)),button("Stop queue",false,()->control("stop_queue",null)));item(board,queue);
        if(client.downloads.length()==0)empty("PC queue is empty","Add a link from here or share one from another app.");
        for(int i=0;i<client.downloads.length();i++){
            JSONObject j=client.downloads.optJSONObject(i);if(j==null)continue;
            String id=j.optString("id"),status=j.optString("status"),name=j.optString("name");
            LinearLayout job=panel();job.addView(strong(name,16));
            long size=j.optLong("total",-1),received=j.optLong("bytes"),speed=(long)j.optDouble("speed",0);
            job.addView(text(cap(status)+" · "+meter(received,size,speed),12,sub));
            if(!j.isNull("error"))job.addView(text(j.optString("error"),12,sub));
            gap(job,7);job.addView(progress(received,size));
            if(status.equals("completed"))actions(job,button("Delete",false,()->deletePc(id)));
            else actions(job,button(status.equals("downloading")||status.equals("connecting")?"Pause":"Resume",true,()->control(status.equals("downloading")||status.equals("connecting")?"pause":"resume",id)),
                button("Stop",false,()->control("stop",id)),button("Delete",false,()->deletePc(id)));
            item(board,job);
        }
    }
    private void deletePc(String id){ask("Delete this queue entry?","The completed file will remain on your PC.","Delete",()->control("remove",id));}
    private void control(String type,String id){run(()->client.control(type,id,null));}
    private void toggleWatch(){
        if(client.watching){stopService(new Intent(this,CompanionService.class));render(true);return;}
        if(Build.VERSION.SDK_INT>=33&&checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)!=PackageManager.PERMISSION_GRANTED){
            requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS},426);
            return;
        }
        startWatching();
    }
    private void startWatching(){
        try{startForegroundService(new Intent(this,CompanionService.class));}catch(IllegalStateException error){message(error.getMessage());}
    }
    @Override public void onRequestPermissionsResult(int request,String[] permissions,int[] grants){
        super.onRequestPermissionsResult(request,permissions,grants);
        if(request==426&&grants.length>0&&grants[0]==PackageManager.PERMISSION_GRANTED)startWatching();
        if(request==BLUETOOTH_PERMISSION){
            if(grants.length>0&&grants[0]==PackageManager.PERMISSION_GRANTED)bluetoothScan();
            else message("Allow Nearby devices to connect to the PC over Bluetooth.");
        }
    }

    private void files(){
        header("File share","Send photos, videos and documents directly between devices.");
        LinearLayout p=panel();p.addView(strong(client.online?"Ready to share":client.pc!=null?"Waiting for PC":"Pair your PC to begin",16));
        p.addView(text("Received files stay in NDM. Tap Open to preview or share them with another app.",13,sub));
        if(client.pc!=null)actions(p,button("Send files",true,()->pick(PICK_FILES)));
        else actions(p,button("Connect PC",true,()->select(1)));
        item(board,p);
        if(!client.transferIssue.isEmpty())empty("Transfer interrupted",client.transferIssue);
        Set<String> shown=new HashSet<>();
        for(int i=0;i<client.transfers.length();i++){JSONObject j=client.transfers.optJSONObject(i);if(j!=null){shown.add(j.optString("id"));transferCard(j);}}
        for(JSONObject staged:client.stagedOffers()){
            if(client.pc==null||!client.pc.pin.equals(staged.optString("pin"))||shown.contains(staged.optString("id")))continue;
            try{staged.put("phase",client.paused(staged.optString("id"))?"paused":"ready").put("to_pc",true);transferCard(staged);}catch(JSONException ignored){}
        }
        if(board.getChildCount()<=3)empty("No local transfers","Use Share to PC in Android’s share menu, or select files here.");
    }
    private void transferCard(JSONObject j){
        String id=j.optString("id"),name=j.optString("name"),phase=j.optString("phase");
        boolean toPc=j.optBoolean("to_pc"),chosen=client.chosen(id);
        if(client.paused(id)&&!phase.equals("complete"))phase="paused";
        boolean awaiting=!toPc&&!chosen&&phase.equals("ready");
        LinearLayout p=panel();p.addView(strong(name,16));
        String description=awaiting?"Ready to receive":phase.equals("ready")?"Transferring or waiting for partner":cap(phase);
        p.addView(text((toPc?"To PC":"From PC")+" · "+description,12,sub));
        long done=j.optLong("bytes"),total=j.optLong("total");
        p.addView(text(meter(done,total,(long)j.optDouble("speed",0)),12,sub));
        if(!j.isNull("error"))p.addView(text(j.optString("error"),12,sub));
        gap(p,7);p.addView(progress(phase.equals("complete")&&total==0?1:done,phase.equals("complete")&&total==0?1:total));
        if(phase.equals("complete")&&!toPc)actions(p,button(name.toLowerCase(Locale.ROOT).endsWith(".apk")?"Review APK":"Open",true,()->openReceived(j)));
        else if(awaiting)actions(p,button("Receive",true,()->run(()->client.accept(id))),button("Decline",false,()->run(()->client.pause(id,false))));
        else if("ready".equals(phase))actions(p,button("Cancel",false,()->run(()->client.pause(id,false))));
        else if(!"complete".equals(phase))actions(p,button("Retry",true,()->run(()->{if(!toPc)client.accept(id);else client.pause(id,true);})));
        item(board,p);
    }
    private void openReceived(JSONObject j){
        try{
            File file=client.received(j);if(!file.isFile())throw new IllegalStateException("This file is no longer stored on your phone.");
            Uri uri=FileProvider.getUriForFile(this,getPackageName()+".files",file);
            if(file.getName().toLowerCase(Locale.ROOT).endsWith(".apk"))inspectApk(uri);else open(uri,file.getName());
        }catch(Exception error){message(error.getMessage());}
    }
    private void open(Uri uri,String name){
        String mime=getContentResolver().getType(uri);
        if(mime==null){
            String ext=name.substring(name.lastIndexOf('.')+1).toLowerCase(Locale.ROOT);
            mime=android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext);
        }
        Intent view=new Intent(Intent.ACTION_VIEW).setDataAndType(uri,mime==null?"application/octet-stream":mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        try{startActivity(Intent.createChooser(view,"Open file"));}catch(ActivityNotFoundException e){message("No app can open this file.");}
    }

    private void usage(){
        header("Download usage",client.online?"Synced with your PC · Internet downloads only":"Phone totals · connect to include your PC");
        LinearLayout p=panel();
        String[] categories={"","video","audio","images","apps","games","archives","documents","other"};
        Spinner time=new Spinner(this);ArrayAdapter<String> timeOptions=new ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,new String[]{"Last 24 hours","Last 7 days","Last 30 days"});time.setAdapter(timeOptions);time.setSelection(period==24?0:period==168?1:2);
        Spinner kind=new Spinner(this);String[] types=Arrays.stream(categories).map(x->x.isEmpty()?"All file types":cap(x)).toArray(String[]::new);
        kind.setAdapter(new ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,types));kind.setSelection(Arrays.asList(categories).indexOf(category));
        LinearLayout filters=new LinearLayout(this);filters.addView(time,new LinearLayout.LayoutParams(0,dp(47),1));filters.addView(kind,new LinearLayout.LayoutParams(0,dp(47),1));p.addView(filters);
        time.setOnItemSelectedListener(chosen(i->{int next=new int[]{24,168,720}[i];if(next!=period){period=next;render(true);}}));
        kind.setOnItemSelectedListener(chosen(i->{if(!categories[i].equals(category)){category=categories[i];render(true);}}));
        int count=period==24?12:period/24,span=period/count;long hour=System.currentTimeMillis()/3600000,first=hour-period+1;
        long[][] bars=new long[count][2];long[] totals=new long[2];
        try{
            JSONArray data=client.online&&client.usage.length()>0?client.usage:client.local.usage();
            for(int i=0;i<data.length();i++){
                JSONObject j=data.getJSONObject(i);long h=j.getLong("hour");if(h<first||h>hour||(!category.isEmpty()&&!category.equals(j.getString("category"))))continue;
                int source=j.getString("device").equals("pc")?0:1;long value=j.getLong("bytes");
                bars[(int)((h-first)/span)][source]+=value;totals[source]+=value;
            }
        }catch(Exception ignored){}
        p.addView(text("Total downloaded",13,sub));p.addView(strong(size(totals[0]+totals[1]),29));
        int blue=color(dark?"#78AAFF":"#1C62CB"),orange=color(dark?"#FFAB75":"#A94314");
        LinearLayout key=new LinearLayout(this);key.addView(text("● PC  "+size(totals[0]),13,blue),new LinearLayout.LayoutParams(0,-2,1));
        key.addView(text("● Mobile  "+size(totals[1]),13,orange),new LinearLayout.LayoutParams(0,-2,1));gap(p,10);p.addView(key);
        if(totals[0]+totals[1]==0){gap(p,14);p.addView(text("No completed downloads in this period.",13,sub));}
        else{
            View graph=new View(this){
                final Paint paint=new Paint(Paint.ANTI_ALIAS_FLAG);
                @Override protected void onDraw(Canvas c){
                    int h=getHeight();float w=getWidth()/(float)count;long max=1;
                    for(long[] bucket:bars)max=Math.max(max,bucket[0]+bucket[1]);
                    paint.setColor(line);for(int i=1;i<=4;i++)c.drawLine(0,h*i/4f,getWidth(),h*i/4f,paint);
                    for(int i=0;i<count;i++){
                        float split=h-bars[i][0]/(float)max*h;float top=split-bars[i][1]/(float)max*h;
                        paint.setColor(blue);c.drawRect(i*w+dp(1),split,(i+1)*w-dp(1),h,paint);
                        paint.setColor(orange);c.drawRect(i*w+dp(1),top,(i+1)*w-dp(1),split,paint);
                    }
                }
            };
            graph.setContentDescription("Download usage timeline. PC "+size(totals[0])+", Mobile "+size(totals[1]));gap(p,12);p.addView(graph,new LinearLayout.LayoutParams(-1,dp(128)));
            DateTimeFormatter date=DateTimeFormatter.ofPattern(period==24?"HH:mm":"MMM d").withZone(ZoneId.systemDefault());
            LinearLayout axis=new LinearLayout(this);axis.addView(text(date.format(Instant.ofEpochSecond(first*3600)),11,sub),new LinearLayout.LayoutParams(0,-2,1));axis.addView(text(date.format(Instant.ofEpochSecond(hour*3600)),11,sub));p.addView(axis);
        }
        item(board,p);
    }
    private AdapterView.OnItemSelectedListener chosen(java.util.function.IntConsumer listener){
        return new AdapterView.OnItemSelectedListener(){public void onItemSelected(AdapterView<?> p,View v,int i,long id){listener.accept(i);}public void onNothingSelected(AdapterView<?> p){}};
    }

    private void addLink(boolean pc){
        EditText field=new EditText(this);field.setSingleLine(true);field.setHint("https://example.com/file.zip");field.setInputType(InputType.TYPE_CLASS_TEXT|InputType.TYPE_TEXT_VARIATION_URI);
        LinearLayout p=panel();p.addView(field);
        AlertDialog dialog=new AlertDialog.Builder(this).setTitle(pc?"Add download on PC":"Add download").setView(p).setNegativeButton("Cancel",null).setPositiveButton(pc?"Download on PC":"Review file",null).show();
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(v->{
            String raw=field.getText().toString().trim();
            try{PhoneDownloads.link(raw);}catch(Exception e){field.setError(e.getMessage());return;}
            dialog.dismiss();
            if(pc)run(()->client.control("add",null,Collections.singletonList(raw)));
            else run(()->{PhoneDownloads.Preview preview=PhoneDownloads.preview(raw);main.post(()->reviewPhone(raw,preview));});
        });
    }
    private void reviewPhone(String url,PhoneDownloads.Preview preview){
        String ext=preview.name().contains(".")?preview.name().substring(preview.name().lastIndexOf('.')):"Unknown";
        new AlertDialog.Builder(this).setTitle(preview.name()).setMessage("Size: "+(preview.size()<0?"Unknown":size(preview.size())+" ("+preview.size()+" B)")+"\nExtension: "+ext+"\nResume: "+preview.resume()+"\nSave to Downloads → NDM")
            .setNegativeButton("Cancel",null).setPositiveButton("Download",(d,b)->run(()->client.local.enqueue(url,preview.name()))).show();
    }

    private void scan(){
        new AlertDialog.Builder(this).setTitle("Choose how to connect")
            .setItems(new String[]{"Bluetooth","Wi-Fi / Internet"},(dialog,choice)->{
                if(choice==0)bluetoothScan();else scanCode(false);
            }).setNegativeButton("Cancel",null).show();
    }
    @SuppressLint("MissingPermission")
    private void bluetoothScan(){
        if(Build.VERSION.SDK_INT>=31&&checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT)!=PackageManager.PERMISSION_GRANTED){
            requestPermissions(new String[]{Manifest.permission.BLUETOOTH_CONNECT},BLUETOOTH_PERMISSION);return;
        }
        BluetoothManager manager=getSystemService(BluetoothManager.class);
        BluetoothAdapter adapter=manager==null?null:manager.getAdapter();
        if(adapter==null){message("Bluetooth is unavailable on this phone.");return;}
        if(adapter.isEnabled())scanCode(true);
        else startActivityForResult(new Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE),ENABLE_BLUETOOTH);
    }
    private void scanCode(boolean bluetooth){
        new IntentIntegrator(this).setRequestCode(bluetooth?SCAN_BLUETOOTH:SCAN_NETWORK).setDesiredBarcodeFormats(IntentIntegrator.QR_CODE)
            .setPrompt(bluetooth?"Scan the Bluetooth QR code in NDM on Windows.":"Scan the Wi-Fi / Internet QR code in NDM on Windows.")
            .setBeepEnabled(false).setOrientationLocked(true).initiateScan();
    }
    private void pick(int request){
        Intent picker=new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*")
            .putExtra(Intent.EXTRA_ALLOW_MULTIPLE,request==PICK_FILES);
        startActivityForResult(picker,request);
    }
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);
        if(request==ENABLE_BLUETOOTH){
            if(result==RESULT_OK)scanCode(true);else message("Bluetooth must be on to use this option.");
            return;
        }
        if(request==SCAN_BLUETOOTH||request==SCAN_NETWORK){
            var scan=IntentIntegrator.parseActivityResult(result,data);
            if(scan.getContents()!=null)pair(scan.getContents(),request==SCAN_BLUETOOTH);
            return;
        }
        if(request==INSTALL_PERMISSION){if(waitingForPermission){waitingForPermission=false;if(getPackageManager().canRequestPackageInstalls())message("Permission enabled. Tap Install to continue.");}return;}
        if(result!=RESULT_OK||data==null)return;
        if(request==PICK_APK&&data.getData()!=null)inspectApk(data.getData());
        if(request==PICK_FILES){
            List<Uri> list=uris(data,false);
            if(!list.isEmpty())confirmShare(list,null);
        }
    }
    private void pair(String scanned,boolean bluetooth){
        try{
            if(PcLink.isBluetoothQr(scanned)!=bluetooth){
                message(bluetooth?"Scan the Bluetooth QR code or choose Wi-Fi / Internet.":"Scan the Wi-Fi / Internet QR code or choose Bluetooth.");
                return;
            }
        }catch(Exception error){message("Scan a pairing QR code from NDM on Windows.");return;}
        CheckBox choice=new CheckBox(this);choice.setText("Remember this device");
        EditText name=new EditText(this);name.setSingleLine(true);name.setText(android.os.Build.MODEL);name.setHint("Device name");
        LinearLayout p=panel();p.addView(name);p.addView(choice);
        new AlertDialog.Builder(this).setTitle(bluetooth?"Connect over Bluetooth":"Connect over Wi-Fi / Internet").setView(p).setNegativeButton("Cancel",null).setPositiveButton("Connect",(d,b)->{
            String label=name.getText().toString().trim();boolean remember=choice.isChecked();
            run(()->{client.pair(scanned,label.isEmpty()?"Android":label,remember);main.post(()->{select(1);message("Paired with PC");if(pendingShare!=null){Intent share=pendingShare;pendingShare=null;handleIntent(share);}});});
        }).show();
    }
    private List<Uri> uris(Intent intent,boolean share){
        LinkedHashSet<Uri> set=new LinkedHashSet<>();
        if(intent.getClipData()!=null)for(int i=0;i<intent.getClipData().getItemCount()&&i<33;i++){Uri uri=intent.getClipData().getItemAt(i).getUri();if(uri!=null)set.add(uri);}
        if(intent.getData()!=null)set.add(intent.getData());
        if(share){
            if(Intent.ACTION_SEND_MULTIPLE.equals(intent.getAction())){
                ArrayList<Uri> extra=Build.VERSION.SDK_INT>=33?intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM,Uri.class):intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM);
                if(extra!=null)for(int i=0;i<extra.size()&&i<33;i++){Uri uri=extra.get(i);if(uri!=null)set.add(uri);}
            }else{
                Uri uri=Build.VERSION.SDK_INT>=33?intent.getParcelableExtra(Intent.EXTRA_STREAM,Uri.class):intent.getParcelableExtra(Intent.EXTRA_STREAM);
                if(uri!=null)set.add(uri);
            }
        }
        set.removeIf(uri->!"content".equals(uri.getScheme()));
        return new ArrayList<>(set);
    }
    private void handleIntent(Intent intent){
        String action=intent.getAction();if(action==null)return;
        try{
            if(Intent.ACTION_VIEW.equals(action)&&intent.getData()!=null){inspectApk(intent.getData());intent.setAction(null);}
            if(Intent.ACTION_SEND.equals(action)||Intent.ACTION_SEND_MULTIPLE.equals(action)){
                List<Uri> files=uris(intent,true);
                CharSequence text=intent.getCharSequenceExtra(Intent.EXTRA_TEXT);
                if(files.isEmpty()&&text==null)return;
                if(client.pc==null){
                    pendingShare=new Intent(intent);select(1);message("Pair your PC to send this item.");
                }else confirmShare(files,text==null?null:text.toString());
                intent.setAction(null);
            }
        }catch(RuntimeException error){message("NDM could not read this shared item.");}
    }
    private void confirmShare(List<Uri> files,String content){
        if(files.size()>32){message("Choose up to 32 files.");return;}
        if(!files.isEmpty()){
            ask("Send "+files.size()+(files.size()==1?" file":" files")+" to PC?","Transfers can continue after a temporary network interruption.","Send",()->{
                select(2);message("Preparing files…");run(()->client.stage(files));
            });
        }else if(content!=null){
            ArrayList<String> links=new ArrayList<>();
            var matcher=java.util.regex.Pattern.compile("(?i)https?://[^\\s<>]+").matcher(content);
            while(matcher.find()&&links.size()<100)try{links.add(PhoneDownloads.link(matcher.group()).toString());}catch(Exception ignored){}
            if(links.isEmpty()){message("No direct download link found in shared text.");return;}
            ask("Download on PC?",links.size()==1?links.get(0):links.size()+" shared links","Download",()->{select(1);run(()->client.control("add",null,links));});
        }
    }

    private void inspectApk(Uri uri){
        if(inspectBusy||installing||downloadingUpdate)return;
        inspectBusy=true;message("Reading APK details…");
        client.commands.execute(()->{
            try{ApkInstall.Details result=ApkInstall.inspect(this,uri);main.post(()->{inspectBusy=false;if(isFinishing())result.file().delete();else showApk(result);});}
            catch(Exception error){main.post(()->{inspectBusy=false;message(error.getMessage());});}
        });
    }
    private void checkUpdate(boolean manual){
        if(checkingUpdate||downloadingUpdate||(!manual&&SystemClock.elapsedRealtime()<nextUpdateCheck))return;
        checkingUpdate=true;
        client.commands.execute(()->{
            try{
                AppUpdates.Update found=AppUpdates.check(this);
                main.post(()->{
                    checkingUpdate=false;nextUpdateCheck=SystemClock.elapsedRealtime()+21600000;
                    if(isDestroyed())return;
                    available=found;
                    if(manual){
                        client.prefs.edit().remove("updateUntil").apply();
                        if(found==null)message("NDM is up to date.");
                    }
                    paintUpdate();
                });
            }catch(Exception ignored){
                main.post(()->{
                    checkingUpdate=false;nextUpdateCheck=SystemClock.elapsedRealtime()+900000;
                    if(manual&&!isDestroyed())message("Could not check for updates. Try again later.");
                });
            }
        });
    }

    private void paintUpdate(){
        updateBanner.removeAllViews();
        boolean snoozed=available!=null&&available.tag().equals(client.prefs.getString("updateTag",""))
            &&System.currentTimeMillis()<client.prefs.getLong("updateUntil",0);
        updateBanner.setVisibility(available==null||snoozed?View.GONE:View.VISIBLE);
        if(updateBanner.getVisibility()!=View.VISIBLE)return;
        updateBanner.addView(strong((downloadingUpdate?"Downloading NDM ":"NDM update available: ")+available.tag(),14));
        if(downloadingUpdate){
            updateBar=progress(0,100);
            updateBanner.addView(updateBar,new LinearLayout.LayoutParams(-1,dp(14)));
        }else{
            AppUpdates.Update seen=available;
            actions(updateBanner,button("What's new",false,()->{
                try{startActivity(new Intent(Intent.ACTION_VIEW,Uri.parse(seen.notes())));}
                catch(ActivityNotFoundException error){message("No browser is available to show the release notes.");}
            }),button("Later",false,()->{
                client.prefs.edit().putString("updateTag",seen.tag()).putLong("updateUntil",System.currentTimeMillis()+86400000).apply();
                paintUpdate();
            }),button("Update",true,()->downloadUpdate(seen)));
        }
    }

    private void downloadUpdate(AppUpdates.Update update){
        if(downloadingUpdate||isDestroyed())return;
        if(inspectBusy||apkDialog!=null||installing){message("Close the APK installer before updating NDM.");return;}
        downloadingUpdate=true;
        paintUpdate();
        client.commands.execute(()->{
            try{
                var details=AppUpdates.download(this,update,percent->main.post(()->{
                    if(downloadingUpdate&&updateBar!=null)updateBar.setProgress(percent*10,true);
                }));
                main.post(()->{
                    downloadingUpdate=false;
                    if(isFinishing()||isDestroyed()){details.file().delete();return;}
                    paintUpdate();showApk(details);
                });
            }catch(Exception error){
                main.post(()->{
                    downloadingUpdate=false;
                    if(!isDestroyed()){paintUpdate();message(error.getMessage());}
                });
            }
        });
    }
    private void showApk(ApkInstall.Details details){
        if(apkDialog!=null)apkDialog.dismiss();
        install=details;Dialog dialog=new Dialog(this);apkDialog=dialog;
        LinearLayout p=panel();p.setPadding(dp(24),dp(23),dp(24),dp(22));p.setGravity(Gravity.CENTER_HORIZONTAL);p.setBackground(shape(surface,30));
        TextView kicker=text("PACKAGE INSTALLER",11,accent);kicker.setLetterSpacing(0.12f);p.addView(kicker);
        gap(p,15);ImageView image=new ImageView(this);image.setImageDrawable(details.icon());image.setContentDescription("App icon");p.addView(image,new LinearLayout.LayoutParams(dp(68),dp(68)));
        gap(p,14);TextView heading=strong(details.label(),25);heading.setGravity(Gravity.CENTER);p.addView(heading);
        TextView packageName=text(details.packageName(),13,sub);packageName.setGravity(Gravity.CENTER);p.addView(packageName);
        gap(p,16);LinearLayout meta=column();meta.setBackground(shape(card,17));meta.setPadding(dp(15),dp(12),dp(15),dp(12));
        meta.addView(text("Version  "+details.version(),14,ink));gap(meta,3);meta.addView(text("APK size  "+size(details.size()),13,sub));p.addView(meta,new LinearLayout.LayoutParams(-1,-2));
        gap(p,12);apkStatus=text("Android will ask you to confirm installation.",12,sub);apkStatus.setGravity(Gravity.CENTER);p.addView(apkStatus);
        apkProgress=progress(0,100);apkProgress.setVisibility(View.GONE);p.addView(apkProgress,new LinearLayout.LayoutParams(-1,dp(10)));
        installButton=button("Install",true,this::beginInstall);actions(p,button("Cancel",false,()->{if(!installing)dialog.dismiss();}),installButton);
        dialog.setOnDismissListener(d->{if(!installing)details.file().delete();if(apkDialog==dialog)apkDialog=null;});
        dialog.setContentView(p);dialog.show();
        Window window=dialog.getWindow();
        if(window!=null){window.setBackgroundDrawable(new ColorDrawable(Color.TRANSPARENT));window.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);window.setDimAmount(0.64f);window.setLayout(Math.min(dp(410),getResources().getDisplayMetrics().widthPixels-dp(32)),-2);}
    }
    private void beginInstall(){
        if(installing||install==null)return;
        if(!getPackageManager().canRequestPackageInstalls()){
            waitingForPermission=true;
            try{startActivityForResult(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,Uri.parse("package:"+getPackageName())),INSTALL_PERMISSION);}
            catch(ActivityNotFoundException e){waitingForPermission=false;message("Android installation permission is unavailable.");}
            return;
        }
        ApkInstall.Details chosen=install;installing=true;installButton.setEnabled(false);apkDialog.setCancelable(false);
        apkStatus.setText("Preparing installation…");apkProgress.setVisibility(View.VISIBLE);
        client.commands.execute(()->{
            try {ApkInstall.stage(this,chosen,percent->main.post(()->{if(apkProgress!=null)apkProgress.setProgress(percent*10,true);}));}
            catch(Exception error){main.post(()->finishInstall(false,chosen.packageName(),error.getMessage()));}
        });
    }
    private void installFeedback(){
        InstallResult.Event event=InstallResult.latest;if(event==null)return;
        InstallResult.latest=null;
        if(event.status()==PackageInstaller.STATUS_PENDING_USER_ACTION){
            if(apkStatus!=null)apkStatus.setText("Waiting for Android confirmation…");
            if(event.confirm()!=null)try{startActivity(event.confirm());}catch(ActivityNotFoundException e){finishInstall(false,event.name(),"Android could not open the confirmation.");}
        }else finishInstall(event.status()==PackageInstaller.STATUS_SUCCESS,event.name(),event.message());
    }
    private void finishInstall(boolean ok,String packageName,String detail){
        installing=false;
        if(apkProgress!=null&&ok)apkProgress.setProgress(1000,true);
        if(apkDialog!=null)apkDialog.dismiss();
        AlertDialog.Builder result=new AlertDialog.Builder(this).setTitle(ok?"App installed":"Installation did not finish")
            .setMessage(ok?packageName:(detail==null?"Android canceled or rejected this APK.":detail))
            .setNegativeButton("Done",null);
        if(ok){
            Intent open=getPackageManager().getLaunchIntentForPackage(packageName);
            if(open!=null)result.setPositiveButton("Open",(dialog,which)->{try{startActivity(open);}catch(ActivityNotFoundException e){message("This app has no launch screen.");}});
        }
        result.show();
    }

    private void ask(String title,String detail,String accept,Runnable work){
        new AlertDialog.Builder(this).setTitle(title).setMessage(detail).setNegativeButton("Cancel",null).setPositiveButton(accept,(d,b)->work.run()).show();
    }
    private void message(String message){if(!isFinishing())Toast.makeText(this,message==null?"Something went wrong.":message,Toast.LENGTH_SHORT).show();}
    interface Job {void work() throws Exception;}
    private void run(Job job){
        client.commands.execute(()->{try{job.work();main.post(()->render(true));}catch(Exception error){main.post(()->message(error.getMessage()));}});
    }
    private static String cap(String string){return string.isEmpty()?string:Character.toUpperCase(string.charAt(0))+string.substring(1);}
    private static String meter(long done,long total,long speed){
        return (total>0?Math.min(100,Math.max(0,done)*100/total)+"% · ":"")+size(done)+(total>=0?" / "+size(total):"")+(speed>0?" · "+size(speed)+"/s":"");
    }
    private static String size(long bytes){
        if(bytes<0)return "—";if(bytes<1024)return bytes+" B";
        String[] units={"KiB","MiB","GiB","TiB","PiB"};double value=bytes/1024d;int i=0;
        while(value>=1024&&i<units.length-1){value/=1024;i++;}
        return String.format(Locale.getDefault(),"%.1f %s",value,units[i]);
    }
}
