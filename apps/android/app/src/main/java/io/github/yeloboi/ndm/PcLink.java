package io.github.yeloboi.ndm;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import org.json.JSONObject;
import java.io.*;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.security.KeyStore;
import java.security.MessageDigest;
import java.security.cert.CertificateException;
import java.security.cert.X509Certificate;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;
import javax.net.ssl.*;

final class PcLink {
    static final int CHUNK = 256 * 1024;
    private static final String ALIAS = "ndm.companion.session";
    final String host;
    final String pin;
    final String token;
    final SSLSocketFactory factory;

    private PcLink(String host, String pin, String token) throws Exception {
        if (!host.matches("(?:[0-9]{1,3}\\.){3}[0-9]{1,3}:42672") || !pin.matches("[a-f0-9]{64}"))
            throw new IOException("Invalid NDM pairing address.");
        for (String part : host.split(":")[0].split("\\.")) if (Integer.parseInt(part) > 255) throw new IOException("Invalid PC address.");
        this.host = host; this.pin = pin; this.token = token;
        byte[] expected = new byte[32];
        for (int i = 0; i < expected.length; i++) expected[i] = (byte) Integer.parseInt(pin.substring(i * 2, i * 2 + 2), 16);
        SSLContext ssl = SSLContext.getInstance("TLS");
        ssl.init(null, new TrustManager[]{ new X509TrustManager() {
            public X509Certificate[] getAcceptedIssuers() { return new X509Certificate[0]; }
            public void checkClientTrusted(X509Certificate[] cert, String auth) throws CertificateException { throw new CertificateException("Client certificate rejected."); }
            public void checkServerTrusted(X509Certificate[] cert, String auth) throws CertificateException {
                try {
                    if (cert == null || cert.length != 1) throw new CertificateException("Unexpected PC certificate.");
                    cert[0].checkValidity();
                    if (!MessageDigest.isEqual(expected, MessageDigest.getInstance("SHA-256").digest(cert[0].getEncoded())))
                        throw new CertificateException("PC identity changed. Scan a new QR code.");
                } catch (Exception error) { throw new CertificateException(error); }
            }
        }}, null);
        factory = ssl.getSocketFactory();
    }

    static PcLink pair(String qr, String id, String name, boolean remember) throws Exception {
        if (qr == null || !qr.startsWith("NDM1:") || qr.length() > 1024) throw new IOException("Scan the QR code from NDM Settings on the PC.");
        JSONObject data = new JSONObject(qr.substring(5));
        if (data.getLong("expires") + 15 < System.currentTimeMillis() / 1000) throw new IOException("This code expired. Generate a fresh code on the PC.");
        PcLink handshake = new PcLink(data.getString("host"), data.getString("pin"), "");
        JSONObject payload = new JSONObject().put("code",data.getString("code")).put("name",name).put("id",id).put("remember",remember);
        String token = handshake.object("POST","/pair",payload).getString("token");
        if (!token.matches("[a-f0-9]{64}")) throw new IOException("Invalid pairing response.");
        return new PcLink(handshake.host,handshake.pin,token);
    }

    JSONObject object(String method, String path, JSONObject data) throws Exception {
        byte[] result = request(method,path,data==null?null:data.toString().getBytes(StandardCharsets.UTF_8),"application/json",8*1024*1024);
        return result.length==0 ? new JSONObject() : new JSONObject(new String(result, StandardCharsets.UTF_8));
    }

    byte[] request(String method, String path, byte[] data, String type, int limit) throws Exception {
        HttpsURLConnection connection=(HttpsURLConnection)new URL("https://"+host+path).openConnection();
        connection.setSSLSocketFactory(factory);
        // The peer's exact QR fingerprint, rather than a DNS hostname, is checked by the trust manager.
        connection.setHostnameVerifier((hostname, session)->hostname.equals(host.split(":")[0]));
        connection.setConnectTimeout(3500); connection.setReadTimeout(6000);
        connection.setInstanceFollowRedirects(false); connection.setUseCaches(false); connection.setRequestMethod(method);
        if(!token.isEmpty())connection.setRequestProperty("Authorization","Bearer "+token);
        try {
            if(data!=null) {
                connection.setDoOutput(true); connection.setRequestProperty("Content-Type",type); connection.setFixedLengthStreamingMode(data.length);
                try(OutputStream out=connection.getOutputStream()){out.write(data);}
            }
            int code=connection.getResponseCode();
            if(code<200||code>=300) {
                String error="PC returned HTTP "+code;
                try(InputStream in=connection.getErrorStream()){
                    if(in!=null) error=new JSONObject(new String(read(in,8192),StandardCharsets.UTF_8)).optString("error",error);
                } catch(Exception ignored){}
                throw new HttpFailure(code,error);
            }
            try(InputStream in=connection.getInputStream()){return read(in,limit);}
        } finally {connection.disconnect();}
    }

    private static byte[] read(InputStream in, int limit) throws IOException {
        ByteArrayOutputStream out=new ByteArrayOutputStream();
        byte[] buffer=new byte[16384]; int length;
        while((length=in.read(buffer))!=-1){if(out.size()+length>limit)throw new IOException("The PC response exceeded its size limit.");out.write(buffer,0,length);}
        return out.toByteArray();
    }

    private static SecretKey localKey() throws Exception {
        KeyStore ks=KeyStore.getInstance("AndroidKeyStore");ks.load(null);
        SecretKey existing=(SecretKey)ks.getKey(ALIAS,null);
        if(existing!=null)return existing;
        KeyGenerator generator=KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore");
        generator.init(new KeyGenParameterSpec.Builder(ALIAS,KeyProperties.PURPOSE_ENCRYPT|KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).setKeySize(256).build());
        return generator.generateKey();
    }

    void save(Context context) throws Exception {
        Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.ENCRYPT_MODE,localKey());
        byte[] encrypted=cipher.doFinal(new JSONObject().put("host",host).put("pin",pin).put("token",token).toString().getBytes(StandardCharsets.UTF_8));
        if(!prefs(context).edit().putString("record",Base64.encodeToString(cipher.getIV(),Base64.NO_WRAP)+":"+Base64.encodeToString(encrypted,Base64.NO_WRAP)).commit())
            throw new IOException("Could not remember the paired PC.");
    }

    static PcLink load(Context context) {
        try {
            String record=prefs(context).getString("record","");
            if(record.isEmpty())return null;
            String[] fields=record.split(":");
            Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.DECRYPT_MODE,localKey(),new GCMParameterSpec(128,Base64.decode(fields[0],Base64.NO_WRAP)));
            JSONObject value=new JSONObject(new String(cipher.doFinal(Base64.decode(fields[1],Base64.NO_WRAP)),StandardCharsets.UTF_8));
            return new PcLink(value.getString("host"),value.getString("pin"),value.getString("token"));
        } catch(Exception error) { forget(context);return null; }
    }

    static void forget(Context context){prefs(context).edit().remove("record").apply();}
    private static SharedPreferences prefs(Context context){return context.getSharedPreferences("pairing",Context.MODE_PRIVATE);}
    static final class HttpFailure extends IOException {
        final int code;
        HttpFailure(int code,String message){super(message);this.code=code;}
    }
}
