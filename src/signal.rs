use crate::config::Config;

use anyhow::{anyhow, Context as _};
use futures_util::{
    stream::{self, Stream},
    FutureExt, StreamExt,
};
use libsignal_protocol::{crypto::DefaultCrypto, Context};
pub use libsignal_service::content::{AttachmentPointer, ContentBody, DataMessage, Metadata};
use serde::{Deserialize, Serialize};
use signal_bot::{config::SledConfigStore, Manager};

use std::path::PathBuf;

#[derive(Clone)]
pub struct SignalClient {
    config: Config,
    manager: Manager<SledConfigStore>,
}

#[derive(Debug)]
pub struct Message {
    pub metadata: Metadata,
    pub body: ContentBody,
}

impl SignalClient {
    pub fn with_config(config: Config) -> Self {
        let config_store = SledConfigStore::new(config.db_path.clone()).unwrap();
        let signal_context = Context::new(DefaultCrypto::default()).unwrap();

        let manager = Manager::with_config_store(config_store, signal_context).unwrap();

        Self { config, manager }
    }

    pub fn get_groups(&self) -> anyhow::Result<Vec<GroupInfo>> {
        // let output = Command::new(self.config.signal_cli.path.as_os_str())
        //     .arg("--username")
        //     .arg(&self.config.user.phone_number)
        //     .arg("listGroups")
        //     .output()?;

        // let res: Result<Vec<_>, anyhow::Error> = output
        //     .stdout
        //     .lines()
        //     .map(|s| {
        //         let s = s?;
        //         let info = GroupInfo::from_str(&s)?;
        //         Ok(info)
        //     })
        //     .collect();
        // res
        Ok(Vec::new())
    }

    pub fn get_contacts(&self) -> anyhow::Result<Vec<ContactInfo>> {
        // let output = Command::new(self.config.signal_cli.path.as_os_str())
        //     .arg("--username")
        //     .arg(&self.config.user.phone_number)
        //     .arg("listContacts")
        //     .output()?;

        // let res: Result<Vec<_>, anyhow::Error> = output
        //     .stdout
        //     .lines()
        //     .map(|s| {
        //         let s = s?;
        //         let info = ContactInfo::from_str(&s)?;
        //         Ok(info)
        //     })
        //     .collect();
        // res
        Ok(Vec::new())
    }

    pub fn stream_messages(self) -> impl Stream<Item = anyhow::Result<Message>> {
        let (tx, rx) = futures::channel::mpsc::channel(32);
        let (stopped_tx, stopped_rx) = tokio::sync::oneshot::channel();
        tokio::task::spawn_local(async move {
            let res = match self.manager.receive_messages(tx).await {
                Ok(()) => Err(anyhow!("stopped receiving messages from Signal")),
                Err(e) => Err(e.into()),
            }
            .context("failed to receive a message from Signal");
            stopped_tx.send(res).expect("logic: stopped channel closed");
        });
        stream::select(
            rx.map(|(metadata, body)| Ok(Message { metadata, body })),
            stopped_rx.into_stream().map(|rx_res| rx_res?),
        )
    }

    pub async fn send_message(&self, message: String, phone_number: String) -> anyhow::Result<()> {
        Ok(self.manager.send_message(phone_number, message).await?)
        // let child = tokio::process::Command::new("dbus-send")
        //     .args(&[
        //         "--session",
        //         "--type=method_call",
        //         "--dest=org.asamk.Signal",
        //         "/org/asamk/Signal",
        //         "org.asamk.Signal.sendMessage",
        //     ])
        //     .arg(format!("string:{}", message))
        //     .arg("array:string:")
        //     .arg(format!("string:{}", phone_number))
        //     .spawn()
        //     .unwrap();

        // tokio::spawn(child);
    }

    pub async fn send_group_message(
        &self,
        _message: String,
        _group_id: String,
    ) -> anyhow::Result<()> {
        log::info!("unimplemented");
        Ok(())
        // let child = tokio::process::Command::new("dbus-send")
        //     .args(&[
        //         "--session",
        //         "--type=method_call",
        //         "--dest=org.asamk.Signal",
        //         "/org/asamk/Signal",
        //         "org.asamk.Signal.sendGroupMessage",
        //     ])
        //     .arg(format!("string:{}", message))
        //     .arg("array:string:")
        //     .arg(format!(
        //         "array:byte:{}",
        //         bytes_to_decimal_list(&base64::decode(&group_id).unwrap())
        //     ))
        //     .spawn()
        //     .unwrap();

        // tokio::spawn(child);
    }

    pub async fn get_contact_name(_phone_number: &str) -> Option<String> {
        None
        // let output = tokio::process::Command::new("dbus-send")
        //     .args(&[
        //         "--session",
        //         "--print-reply",
        //         "--type=method_call",
        //         "--dest=org.asamk.Signal",
        //         "/org/asamk/Signal",
        //         "org.asamk.Signal.getContactName",
        //     ])
        //     .arg(format!("string:{}", phone_number))
        //     .output();

        // let output = output.await.ok()?;
        // extract_dbus_string_response(&output.stdout).filter(|s| !String::is_empty(s))
    }

    pub async fn get_group_name(_group_id: &str) -> Option<String> {
        None
        // let output = tokio::process::Command::new("dbus-send")
        //     .args(&[
        //         "--session",
        //         "--print-reply",
        //         "--type=method_call",
        //         "--dest=org.asamk.Signal",
        //         "/org/asamk/Signal",
        //         "org.asamk.Signal.getGroupName",
        //     ])
        //     .arg(format!(
        //         "array:byte:{}",
        //         bytes_to_decimal_list(&base64::decode(group_id.as_bytes()).ok()?)
        //     ))
        //     .output();

        // let output = output.await.ok()?;
        // extract_dbus_string_response(&output.stdout).filter(|s| !String::is_empty(s))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    pub source: String,
    pub source_device: u8,
    // pub relay: Option<?>,
    pub timestamp: u64,
    pub message: Option<String>,
    pub is_receipt: bool,
    pub is_read: Option<bool>,
    pub is_delivery: Option<bool>,
    pub data_message: Option<InnerMessage>,
    pub sync_message: Option<SyncMessage>,
    // call_message
    // receipt_message
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerMessage {
    pub timestamp: u64,
    pub message: Option<String>,
    pub expires_in_seconds: u64,
    pub attachments: Option<Vec<Attachment>>,
    pub group_info: Option<GroupInfo>,
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMessage {
    pub sent_message: InnerMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfo {
    pub group_id: String,
    // members
    pub name: Option<String>,
    pub members: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub content_type: String,
    pub filename: PathBuf,
    pub size: u64,
}

#[derive(Debug)]
pub struct ContactInfo {
    pub name: String,
    pub phone_number: String,
}
