package io.github.yeloboi.ndm;

import android.annotation.SuppressLint;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothManager;
import android.bluetooth.BluetoothSocket;
import android.content.Context;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.util.UUID;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

@SuppressLint("MissingPermission")
final class BluetoothTransport {
    private static final UUID SERVICE=UUID.fromString("448a7198-603d-43a1-baf1-99e4fa08545b");
    private static final ScheduledExecutorService TIMEOUT=Executors.newSingleThreadScheduledExecutor(task->daemon("NDM Bluetooth timeout",task));
    private final BluetoothAdapter adapter;
    final String address;

    BluetoothTransport(Context context,String address)throws IOException{
        if(!BluetoothAdapter.checkBluetoothAddress(address))throw new IOException("Invalid PC Bluetooth address.");
        BluetoothManager manager=context==null?null:context.getSystemService(BluetoothManager.class);
        adapter=manager==null?null:manager.getAdapter();
        this.address=address;
    }

    interface Exchange<T>{T request(String host)throws Exception;}

    synchronized <T>T exchange(Exchange<T> exchange)throws Exception{
        if(adapter==null)throw new IOException("Bluetooth is unavailable on this phone.");
        try{
            if(!adapter.isEnabled())throw new IOException("Turn on Bluetooth on the phone and PC.");
            try(BluetoothSocket radio=adapter.getRemoteDevice(address).createRfcommSocketToServiceRecord(SERVICE)){
                var watchdog=TIMEOUT.schedule(()->close(radio),45,TimeUnit.SECONDS);
                try{radio.connect();}
                catch(IOException error){
                    throw new IOException("Bluetooth could not reach NDM. Keep NDM open; if asked, pair the devices in Bluetooth settings, then try again.",error);
                }finally{watchdog.cancel(false);}
                try(ServerSocket loopback=new ServerSocket(0,1,InetAddress.getByName("127.0.0.1"))){
                    loopback.setSoTimeout(6000);
                    daemon("NDM Bluetooth tunnel",()->bridge(loopback,radio)).start();
                    var limit=TIMEOUT.schedule(()->close(radio),60,TimeUnit.SECONDS);
                    try{return exchange.request("127.0.0.1:"+loopback.getLocalPort());}
                    finally{limit.cancel(false);}
                }
            }
        }catch(SecurityException error){
            throw new IOException("Allow Nearby devices for NDM in Android settings.",error);
        }
    }

    private static void bridge(ServerSocket listener,BluetoothSocket radio){
        try(Socket local=listener.accept()){
            daemon("NDM Bluetooth upload",()->{
                try{copy(local.getInputStream(),radio.getOutputStream());}
                catch(IOException ignored){}
                finally{close(radio);}
            }).start();
            copy(radio.getInputStream(),local.getOutputStream());
        }catch(IOException ignored){}
        finally{close(radio);}
    }

    private static void copy(InputStream source,OutputStream target)throws IOException{
        byte[] buffer=new byte[8192];
        int length;
        while((length=source.read(buffer))!=-1)target.write(buffer,0,length);
    }

    private static Thread daemon(String name,Runnable task){
        Thread thread=new Thread(task,name);thread.setDaemon(true);return thread;
    }

    private static void close(BluetoothSocket socket){
        try{socket.close();}catch(IOException ignored){}
    }
}
