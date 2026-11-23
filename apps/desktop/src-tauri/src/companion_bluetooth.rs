use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{oneshot, Semaphore},
};
use windows::{
    core::{Result, GUID},
    Devices::{
        Bluetooth::{
            BluetoothAdapter,
            Rfcomm::{RfcommServiceId, RfcommServiceProvider},
        },
        Radios::{Radio, RadioAccessStatus, RadioState},
    },
    Foundation::TypedEventHandler,
    Networking::Sockets::{
        SocketProtectionLevel, StreamSocket, StreamSocketListener,
        StreamSocketListenerConnectionReceivedEventArgs,
    },
    Storage::Streams::{DataReader, DataWriter, InputStreamOptions},
};

const SERVICE_ID: GUID = GUID::from_u128(0x448a7198_603d_43a1_baf1_99e4fa08545b);

pub struct Listener {
    provider: RfcommServiceProvider,
    socket: StreamSocketListener,
    event: i64,
    pub address: String,
}

impl Listener {
    pub async fn start(
        port: u16,
        app: Option<tauri::AppHandle>,
    ) -> std::result::Result<Self, String> {
        const RADIO_HELP: &str = "Turn on Bluetooth in Windows Settings, then try again.";
        let adapter = BluetoothAdapter::GetDefaultAsync()
            .map_err(message)?
            .await
            .map_err(|_| "No working Bluetooth adapter was found on this PC.".to_string())?;
        if !adapter.IsClassicSupported().map_err(message)? {
            return Err("This PC does not support Bluetooth Classic.".into());
        }
        let radio = adapter
            .GetRadioAsync()
            .map_err(message)?
            .await
            .map_err(message)?;
        if radio.State().map_err(message)? != RadioState::On {
            let app = app.ok_or(RADIO_HELP)?;
            let (send, receive) = oneshot::channel();
            app.run_on_main_thread(move || {
                let _ = send.send(Radio::RequestAccessAsync());
            })
            .map_err(message)?;
            let access = receive.await.map_err(message)?.map_err(message)?;
            if access.await.map_err(message)? != RadioAccessStatus::Allowed
                || radio
                    .SetStateAsync(RadioState::On)
                    .map_err(message)?
                    .await
                    .map_err(message)?
                    != RadioAccessStatus::Allowed
            {
                return Err(RADIO_HELP.into());
            }
        }
        let address = adapter.BluetoothAddress().map_err(message)?.to_be_bytes()[2..]
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        Self::advertise(port, address).await.map_err(message)
    }

    async fn advertise(port: u16, address: String) -> Result<Self> {
        let id = RfcommServiceId::FromUuid(SERVICE_ID)?;
        let provider = RfcommServiceProvider::CreateAsync(&id)?.await?;
        let socket = StreamSocketListener::new()?;
        let runtime = tokio::runtime::Handle::current();
        let sessions = Arc::new(Semaphore::new(4));
        let event = socket.ConnectionReceived(&TypedEventHandler::<
            StreamSocketListener,
            StreamSocketListenerConnectionReceivedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                let connection = args.Socket()?;
                match Arc::clone(&sessions).try_acquire_owned() {
                    Ok(slot) => {
                        runtime.spawn(async move {
                            let _slot = slot;
                            let _ = tokio::time::timeout(
                                Duration::from_secs(90),
                                relay(&connection, port),
                            )
                            .await;
                            let _ = connection.Close();
                        });
                    }
                    Err(_) => {
                        let _ = connection.Close();
                    }
                }
            }
            Ok(())
        }))?;
        let listener = Self {
            provider,
            socket,
            event,
            address,
        };
        listener
            .socket
            .BindServiceNameWithProtectionLevelAsync(
                &id.AsString()?,
                SocketProtectionLevel::BluetoothEncryptionWithAuthentication,
            )?
            .await?;
        listener.provider.StartAdvertising(&listener.socket)?;
        Ok(listener)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.provider.StopAdvertising();
        let _ = self.socket.RemoveConnectionReceived(self.event);
        let _ = self.socket.Close();
    }
}

async fn relay(bluetooth: &StreamSocket, port: u16) -> std::result::Result<(), String> {
    let tcp = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(message)?;
    tcp.set_nodelay(true).map_err(message)?;
    let (mut from_pc, mut to_pc) = tcp.into_split();
    let reader = DataReader::CreateDataReader(&bluetooth.InputStream().map_err(message)?)
        .map_err(message)?;
    reader
        .SetInputStreamOptions(InputStreamOptions::Partial)
        .map_err(message)?;
    let writer = DataWriter::CreateDataWriter(&bluetooth.OutputStream().map_err(message)?)
        .map_err(message)?;
    tokio::select! {
        result = async {
            let mut buffer = [0; 8192];
            loop {
                let size = reader.LoadAsync(8192).map_err(message)?.await.map_err(message)? as usize;
                if size == 0 {
                    return Ok(());
                }
                reader.ReadBytes(&mut buffer[..size]).map_err(message)?;
                to_pc.write_all(&buffer[..size]).await.map_err(message)?;
            }
        } => result,
        result = async {
            let mut buffer = [0; 8192];
            loop {
                let size = from_pc.read(&mut buffer).await.map_err(message)?;
                if size == 0 {
                    return Ok(());
                }
                writer.WriteBytes(&buffer[..size]).map_err(message)?;
                if writer.StoreAsync().map_err(message)?.await.map_err(message)? as usize != size {
                    return Err("Incomplete Bluetooth write.".into());
                }
            }
        } => result,
    }
}

fn message(error: impl std::fmt::Display) -> String {
    format!("Bluetooth: {error}")
}
