use std::time::Duration;

use deno_broadcast_channel::BroadcastChannel;
use serde::{de::DeserializeOwned, Serialize};

use crate::{big_json_args, Error, Runtime};

/// Helper struct to wrap a broadcast channel
/// Takes care of some of the boilerplate for serialization/deserialization
pub struct BroadcastChannelWrapper<Channel: BroadcastChannel> {
    channel: Channel,
    resource: <Channel as BroadcastChannel>::Resource,
    name: String,
}
impl<Channel: BroadcastChannel> BroadcastChannelWrapper<Channel> {
    /// Create a new broadcast channel wrapper and subscribe to the channel
    /// Unsubscribe is called when the wrapper is dropped
    ///
    /// # Errors
    /// Will return an error if the channel cannot be subscribed to
    pub fn new(channel: &Channel, name: impl ToString) -> Result<Self, Error> {
        let channel = channel.clone();
        let resource = channel.subscribe()?;
        let name = name.to_string();
        Ok(Self {
            channel,
            resource,
            name,
        })
    }

    /// Send a message to the channel, blocking until the message is sent
    ///
    /// # Errors
    /// Will return an error if the message cannot be serialized or sent
    pub fn send_sync<T: Serialize>(&self, runtime: &mut Runtime, data: T) -> Result<(), Error> {
        let tokio_rt = runtime.tokio_runtime();
        tokio_rt.block_on(self.send(runtime, data))
    }

    /// Send a message to the channel
    ///
    /// # Errors
    /// Will return an error if the message cannot be serialized or sent
    pub async fn send<T: Serialize>(&self, runtime: &mut Runtime, data: T) -> Result<(), Error> {
        let data: Vec<u8> = runtime
            .call_function_async(None, "broadcast_serialize", &data)
            .await?;
        self.channel
            .send(&self.resource, self.name.clone(), data)
            .await?;
        Ok(())
    }

    /// Receive a message from the channel, waiting for a message to arrive, or until the timeout is reached
    ///
    /// # Errors
    /// Will return an error if the message cannot be deserialized
    /// or if receiving the message fails
    pub async fn recv<T: DeserializeOwned>(
        &self,
        runtime: &mut Runtime,
        timeout: Option<Duration>,
    ) -> Result<Option<T>, Error> {
        let msg = if let Some(timeout) = timeout {
            tokio::select! {
                msg = self.channel.recv(&self.resource) => msg,
                () = tokio::time::sleep(timeout) => Ok(None),
            }
        } else {
            self.channel.recv(&self.resource).await
        }?;

        let Some((name, data)) = msg else {
            return Ok(None);
        };

        if name == self.name {
            let data: T = runtime
                .call_function_async(None, "broadcast_deserialize", big_json_args!(data))
                .await?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Receive a message from the channel, blocking until a message arrives, or until the timeout is reached
    ///
    /// # Errors
    /// Will return an error if the message cannot be deserialized
    /// or if receiving the message fails
    pub fn recv_sync<T: DeserializeOwned>(
        &self,
        runtime: &mut Runtime,
        timeout: Option<Duration>,
    ) -> Result<Option<T>, Error> {
        let tokio_rt = runtime.tokio_runtime();
        tokio_rt.block_on(self.recv(runtime, timeout))
    }
}

impl<Channel: BroadcastChannel> Drop for BroadcastChannelWrapper<Channel> {
    fn drop(&mut self) {
        self.channel.unsubscribe(&self.resource).ok();
    }
}
