---
title: Send and Receive Messages in a Reliable Channel
hide_table_of_contents: true
displayed_sidebar: build
---

Learn how to send and receive messages with a convenient SDK that provide various reliable functionalities out-of-the-box.

:::warning
This is an experimental feature and has a number of [limitations](https://github.com/waku-org/js-waku/pull/2526).
:::

## Import Waku SDK

```shell
npm install @waku/sdk@latest
```

Or using a CDN, note this is an ESM package so `type="module"` is needed.

```html
<script type="module">
  import {
    createLightNode,
    ReliableChannel
  } from 'https://unpkg.com/@waku/sdk@latest/bundle/index.js';

  // Your code here
  
</script>
```

## Create a Waku node

Use the `createLightNode()` function to create a [Light Node](/learn/glossary#light-node) and interact with the Waku Network:

```js
import { createLightNode } from "@waku/sdk";

// Create a Light Node
const node = await createLightNode({ defaultBootstrap: true });
```

:::info
When the `defaultBootstrap` parameter is set to `true`, your node will be bootstrapped using the [default bootstrap method](/build/javascript/configure-discovery#default-bootstrap-method). Have a look at the [Bootstrap Nodes and Discover Peers](/build/javascript/configure-discovery) guide to learn more methods to bootstrap nodes.
:::

## Create encoders and decoders

Choose a [content topic](/learn/concepts/content-topics) for your application and create a message `encoder` and `decoder`:

```js
// Choose a content topic
const ct = "/my-app/1/messages/proto";

// Create a message encoder and decoder
const encoder = node.createEncoder({ contentTopic: ct });
const decoder = node.createDecoder({ contentTopic: ct });
```

You can also use [`@waku/message-encryption`](/build/javascript/message-encryption) to encrypt and decrypt messages using Waku libraries.

:::info
In this example, users send and receive messages on a shared content topic. However, real applications may have users broadcasting messages while others listen or only have 1:1 exchanges. Waku supports all these use cases.
:::

## Listen for connection status

The Waku node will emit `health` events to help you know whether the node is connected to the network.
This can be useful to give feedback to the user, or stop some action (e.g. sending messages) when offline:

```js
import { HealthStatus } from "@waku/sdk";

node.events.addEventListener("waku:health", (event) => {
    const health = event.detail;
    
    if (health === HealthStatus.SufficientlyHealthy) {
        // Show to the user they are connected
    } else if (status === HealthStatus.MinimallyHealthy) {
        // Maybe put some clue to the user that while we are connected,
        // there may be issues sending or receiving messages
    } else {
        // Show to the user they are disconnected from the network
    }
});
```

## Create a reliable channel

You need to choose a channel name: it acts as an identifier to the conversation, participants will try to ensure they all have the same
messages within a given channel.

```js
const channelName = "channel-number-15"
```

Finally, each participant need to identify themselves for reliability purposes, so they can confirm _others_ have received their messages.

It is up to you how to generate an id. Every participant **must** have a different id.

```js
const senderId = generateRandomStringId();
```

You can now create a reliable channel:

```js
import { ReliableChannel } from "@waku/sdk";

const reliableChannel = await ReliableChannel.create(node, channelName, senderId, encoder, decoder)
```

The channel will automatically start the Waku node and fetch messages.

## Create a message structure

Create your application's message structure using [Protobufjs](https://github.com/protobufjs/protobuf.js#usage):

```js
import protobuf from "protobufjs";

// Create a message structure using Protobuf
const DataPacket = new protobuf.Type("DataPacket")
  .add(new protobuf.Field("timestamp", 1, "uint64"))
  .add(new protobuf.Field("sender", 2, "string"))
  .add(new protobuf.Field("message", 3, "string"));
```

:::info
Have a look at the [Protobuf installation](/build/javascript/#message-structure) guide for adding the `protobufjs` package to your project.
:::

## Listen to incoming messages

The reliable channel will emit incoming messages. To process them, simply add a listener:

```js
reliableChannel.addEventListener("message-received", (event) => {
  const wakuMessage = event.detail;
  
  // decode your payload using the protobuf object previously created
  const { timestamp, sender, message } = DataPacket.decode(wakuMessage.payload);
  
  // ... process the message as you wish
})
```

## Monitor sync status

The reliable channel provides sync status events to help you understand whether the channel is fully synchronized with all participants. This is useful for showing loading states, alerting users about missing messages, or detecting permanently lost messages.

### Understanding sync states

The channel can be in one of two states:

- **`synced`**: The channel is not aware of any missing messages that can still be retrieved. Note that some messages may have been permanently lost (see `lost` in the status detail).
- **`syncing`**: The channel is aware of missing messages and is attempting to retrieve them.

### Status detail structure

Each sync status event includes a `StatusDetail` object with:

- **`received`**: Number of messages successfully received
- **`missing`**: Number of messages that are missing but may still be retrievable
- **`lost`**: Number of messages considered permanently lost (irretrievable). Messages are marked as lost when they cannot be retrieved within a time constraint.

### Listen to sync status events

```js
// Listen for when the channel is fully synced
reliableChannel.syncStatus.addEventListener("synced", (event) => {
  const { received, missing, lost } = event.detail;

  console.log(`Channel synced: ${received} messages received`);

  if (lost > 0) {
    // Alert the user that some messages were permanently lost
    console.warn(`Warning: ${lost} messages could not be retrieved`);
  }

  // Hide loading spinner, show "up to date" indicator, etc.
});

// Listen for when the channel is syncing
reliableChannel.syncStatus.addEventListener("syncing", (event) => {
  const { received, missing, lost } = event.detail;

  console.log(`Syncing: ${missing} messages are being retrieved...`);

  // Show loading spinner, "syncing" indicator, etc.
});
```

:::warning
When messages are marked as permanently lost, there is currently no automatic recovery mechanism available. You can inform users about the loss, but the messages cannot be retrieved. As we continue to improve the Reliable Channel feature, we may add additional recovery mechanisms in the future.
:::

## Send messages

To send messages in the reliable channel, encode the message in a payload.

```js
// Create a new message object
const protoMessage = DataPacket.create({
  timestamp: Date.now(),
  sender: "Alice",
  message: "Hello, World!",
});

// Serialise the message using Protobuf
const serialisedMessage = DataPacket.encode(protoMessage).finish();
```

Then, send the message and setup listeners so you can know when the message:
- has been sent
- has been acknowledged by other participants in the channel
- has encountered an error

```js
// Send the message, and get the id to track events
const messageId = reliableChannel.send(payload);
        
reliableChannel.addEventListener("sending-message-irrecoverable-error", (event) => {
    if (messageId === event.detail.messageId) {
      console.error('Failed to send message:', event.detail.error);
      // Show an error to the user
    }
})

reliableChannel.addEventListener("message-sent", (event) => {
    if (messageId === event.detail) {
        // Message sent, show '✔' to the user, etc
    }
})

reliableChannel.addEventListener("message-acknowledged", (event) => {
  if (messageId === event.detail) {
    // Message acknowledged by other participants, show '✔✔' to the user, etc
  }
})
```

:::tip Congratulations!
You have successfully sent and received messages over the Waku Network using our reliable protocols such as Scalable Data Sync (SDS) and P2P Reliability.
:::

---
title: Send and Receive Messages in a Reliable Channel
hide_table_of_contents: true
displayed_sidebar: build
---

Learn how to send and receive messages with a convenient SDK that provide various reliable functionalities out-of-the-box.

:::warning
This is an experimental feature and has a number of [limitations](https://github.com/waku-org/js-waku/pull/2526).
:::

## Import Waku SDK

```shell
npm install @waku/sdk@latest
```

Or using a CDN, note this is an ESM package so `type="module"` is needed.

```html
<script type="module">
  import {
    createLightNode,
    ReliableChannel
  } from 'https://unpkg.com/@waku/sdk@latest/bundle/index.js';

  // Your code here
  
</script>
```

## Create a Waku node

Use the `createLightNode()` function to create a [Light Node](/learn/glossary#light-node) and interact with the Waku Network:

```js
import { createLightNode } from "@waku/sdk";

// Create a Light Node
const node = await createLightNode({ defaultBootstrap: true });
```

:::info
When the `defaultBootstrap` parameter is set to `true`, your node will be bootstrapped using the [default bootstrap method](/build/javascript/configure-discovery#default-bootstrap-method). Have a look at the [Bootstrap Nodes and Discover Peers](/build/javascript/configure-discovery) guide to learn more methods to bootstrap nodes.
:::

## Create encoders and decoders

Choose a [content topic](/learn/concepts/content-topics) for your application and create a message `encoder` and `decoder`:

```js
// Choose a content topic
const ct = "/my-app/1/messages/proto";

// Create a message encoder and decoder
const encoder = node.createEncoder({ contentTopic: ct });
const decoder = node.createDecoder({ contentTopic: ct });
```

You can also use [`@waku/message-encryption`](/build/javascript/message-encryption) to encrypt and decrypt messages using Waku libraries.

:::info
In this example, users send and receive messages on a shared content topic. However, real applications may have users broadcasting messages while others listen or only have 1:1 exchanges. Waku supports all these use cases.
:::

## Listen for connection status

The Waku node will emit `health` events to help you know whether the node is connected to the network.
This can be useful to give feedback to the user, or stop some action (e.g. sending messages) when offline:

```js
import { HealthStatus } from "@waku/sdk";

node.events.addEventListener("waku:health", (event) => {
    const health = event.detail;
    
    if (health === HealthStatus.SufficientlyHealthy) {
        // Show to the user they are connected
    } else if (status === HealthStatus.MinimallyHealthy) {
        // Maybe put some clue to the user that while we are connected,
        // there may be issues sending or receiving messages
    } else {
        // Show to the user they are disconnected from the network
    }
});
```

## Create a reliable channel

You need to choose a channel name: it acts as an identifier to the conversation, participants will try to ensure they all have the same
messages within a given channel.

```js
const channelName = "channel-number-15"
```

Finally, each participant need to identify themselves for reliability purposes, so they can confirm _others_ have received their messages.

It is up to you how to generate an id. Every participant **must** have a different id.

```js
const senderId = generateRandomStringId();
```

You can now create a reliable channel:

```js
import { ReliableChannel } from "@waku/sdk";

const reliableChannel = await ReliableChannel.create(node, channelName, senderId, encoder, decoder)
```

The channel will automatically start the Waku node and fetch messages.

## Create a message structure

Create your application's message structure using [Protobufjs](https://github.com/protobufjs/protobuf.js#usage):

```js
import protobuf from "protobufjs";

// Create a message structure using Protobuf
const DataPacket = new protobuf.Type("DataPacket")
  .add(new protobuf.Field("timestamp", 1, "uint64"))
  .add(new protobuf.Field("sender", 2, "string"))
  .add(new protobuf.Field("message", 3, "string"));
```

:::info
Have a look at the [Protobuf installation](/build/javascript/#message-structure) guide for adding the `protobufjs` package to your project.
:::

## Listen to incoming messages

The reliable channel will emit incoming messages. To process them, simply add a listener:

```js
reliableChannel.addEventListener("message-received", (event) => {
  const wakuMessage = event.detail;
  
  // decode your payload using the protobuf object previously created
  const { timestamp, sender, message } = DataPacket.decode(wakuMessage.payload);
  
  // ... process the message as you wish
})
```

## Monitor sync status

The reliable channel provides sync status events to help you understand whether the channel is fully synchronized with all participants. This is useful for showing loading states, alerting users about missing messages, or detecting permanently lost messages.

### Understanding sync states

The channel can be in one of two states:

- **`synced`**: The channel is not aware of any missing messages that can still be retrieved. Note that some messages may have been permanently lost (see `lost` in the status detail).
- **`syncing`**: The channel is aware of missing messages and is attempting to retrieve them.

### Status detail structure

Each sync status event includes a `StatusDetail` object with:

- **`received`**: Number of messages successfully received
- **`missing`**: Number of messages that are missing but may still be retrievable
- **`lost`**: Number of messages considered permanently lost (irretrievable). Messages are marked as lost when they cannot be retrieved within a time constraint.

### Listen to sync status events

```js
// Listen for when the channel is fully synced
reliableChannel.syncStatus.addEventListener("synced", (event) => {
  const { received, missing, lost } = event.detail;

  console.log(`Channel synced: ${received} messages received`);

  if (lost > 0) {
    // Alert the user that some messages were permanently lost
    console.warn(`Warning: ${lost} messages could not be retrieved`);
  }

  // Hide loading spinner, show "up to date" indicator, etc.
});

// Listen for when the channel is syncing
reliableChannel.syncStatus.addEventListener("syncing", (event) => {
  const { received, missing, lost } = event.detail;

  console.log(`Syncing: ${missing} messages are being retrieved...`);

  // Show loading spinner, "syncing" indicator, etc.
});
```

:::warning
When messages are marked as permanently lost, there is currently no automatic recovery mechanism available. You can inform users about the loss, but the messages cannot be retrieved. As we continue to improve the Reliable Channel feature, we may add additional recovery mechanisms in the future.
:::

## Send messages

To send messages in the reliable channel, encode the message in a payload.

```js
// Create a new message object
const protoMessage = DataPacket.create({
  timestamp: Date.now(),
  sender: "Alice",
  message: "Hello, World!",
});

// Serialise the message using Protobuf
const serialisedMessage = DataPacket.encode(protoMessage).finish();
```

Then, send the message and setup listeners so you can know when the message:
- has been sent
- has been acknowledged by other participants in the channel
- has encountered an error

```js
// Send the message, and get the id to track events
const messageId = reliableChannel.send(payload);
        
reliableChannel.addEventListener("sending-message-irrecoverable-error", (event) => {
    if (messageId === event.detail.messageId) {
      console.error('Failed to send message:', event.detail.error);
      // Show an error to the user
    }
})

reliableChannel.addEventListener("message-sent", (event) => {
    if (messageId === event.detail) {
        // Message sent, show '✔' to the user, etc
    }
})

reliableChannel.addEventListener("message-acknowledged", (event) => {
  if (messageId === event.detail) {
    // Message acknowledged by other participants, show '✔✔' to the user, etc
  }
})
```

:::tip Congratulations!
You have successfully sent and received messages over the Waku Network using our reliable protocols such as Scalable Data Sync (SDS) and P2P Reliability.
:::

---
title: Send and Receive Messages Using Light Push and Filter
hide_table_of_contents: true
displayed_sidebar: build
---

This guide provides detailed steps to start using the `@waku/sdk` package by setting up a [Light Node](/learn/glossary#light-node) to send messages using the [Light Push protocol](/learn/concepts/protocols#light-push), and receive messages using the [Filter protocol](/learn/concepts/protocols#filter). Have a look at the [installation guide](/build/javascript/#installation) for steps on adding `@waku/sdk` to your project.

## Create a light node

Use the `createLightNode()` function to create a [Light Node](/learn/glossary#light-node) and interact with the Waku Network:

```js
import { createLightNode } from "@waku/sdk";

// Create and start a Light Node
const node = await createLightNode({ defaultBootstrap: true });
await node.start();

// Use the stop() function to stop a running node
// await node.stop();
```

:::info
When the `defaultBootstrap` parameter is set to `true`, your node will be bootstrapped using the [default bootstrap method](/build/javascript/configure-discovery#default-bootstrap-method). Have a look at the [Bootstrap Nodes and Discover Peers](/build/javascript/configure-discovery) guide to learn more methods to bootstrap nodes.
:::

A node needs to know how to route messages. By default, it will use The Waku Network configuration (`{ clusterId: 1, shards: [0,1,2,3,4,5,6,7] }`). For most applications, it's recommended to use autosharding:

```js
// Create node with auto sharding (recommended)
const node = await createLightNode({
  defaultBootstrap: true,
  networkConfig: {
    clusterId: 1,
    contentTopics: ["/my-app/1/notifications/proto"],
  },
});
```

### Alternative network configuration

If your project requires a specific network configuration, you can use static sharding:

```js
// Create node with static sharding
const node = await createLightNode({
  defaultBootstrap: true,
  networkConfig: {
    clusterId: 1,
    shards: [0, 1, 2, 3],
  },
});
```

## Connect to remote peers

Use the `node.waitForPeers()` function to wait for the node to connect with peers on the Waku Network:

```js
// Wait for a successful peer connection
await node.waitForPeers();
```

The `protocols` parameter allows you to specify the [protocols](/learn/concepts/protocols) that the remote peers should have enabled:

```js
import { Protocols } from "@waku/sdk";

// Wait for peer connections with specific protocols
await node.waitForPeers([Protocols.LightPush, Protocols.Filter]);
```

## Choose a content topic

Choose a [content topic](/learn/concepts/content-topics) for your application and create a message `encoder` and `decoder`:

```js
import { createEncoder, createDecoder } from "@waku/sdk";

// Choose a content topic
const contentTopic = "/light-guide/1/message/proto";

// Create a message encoder and decoder
const encoder = createEncoder({ contentTopic });
const decoder = createDecoder(contentTopic);
```

The `ephemeral` parameter allows you to specify whether messages should **NOT** be stored by [Store peers](/build/javascript/store-retrieve-messages):

```js
const encoder = createEncoder({
  contentTopic: contentTopic, // message content topic
  ephemeral: true, // allows messages NOT be stored on the network
});
```

The `pubsubTopicShardInfo` parameter allows you to configure a different network configuration for your `encoder` and `decoder`:

```js
// Create the network config
const networkConfig = { clusterId: 3, shards: [1, 2] };

// Create encoder and decoder with custom network config
const encoder = createEncoder({
  contentTopic: contentTopic,
  pubsubTopicShardInfo: networkConfig,
});
const decoder = createDecoder(contentTopic, networkConfig);
```

:::info
In this example, users send and receive messages on a shared content topic. However, real applications may have users broadcasting messages while others listen or only have 1:1 exchanges. Waku supports all these use cases.
:::

## Create a message structure

Create your application's message structure using [Protobuf's valid message](https://github.com/protobufjs/protobuf.js#usage) fields:

```js
import protobuf from "protobufjs";

// Create a message structure using Protobuf
const DataPacket = new protobuf.Type("DataPacket")
  .add(new protobuf.Field("timestamp", 1, "uint64"))
  .add(new protobuf.Field("sender", 2, "string"))
  .add(new protobuf.Field("message", 3, "string"));
```

:::info
Have a look at the [Protobuf installation](/build/javascript/#message-structure) guide for adding the `protobufjs` package to your project.
:::

## Send messages using light push

To send messages over the Waku Network using the `Light Push` protocol, create a new message object and use the `lightPush.send()` function:

```js
// Create a new message object
const protoMessage = DataPacket.create({
  timestamp: Date.now(),
  sender: "Alice",
  message: "Hello, World!",
});

// Serialise the message using Protobuf
const serialisedMessage = DataPacket.encode(protoMessage).finish();

// Send the message using Light Push
await node.lightPush.send(encoder, {
  payload: serialisedMessage,
});
```

## Receive messages using filter

To receive messages using the `Filter` protocol, create a callback function for message processing, then use the `filter.subscribe()` function to subscribe to a `content topic`:

```js
// Create the callback function
const callback = (wakuMessage) => {
  // Check if there is a payload on the message
  if (!wakuMessage.payload) return;
  // Render the messageObj as desired in your application
  const messageObj = DataPacket.decode(wakuMessage.payload);
  console.log(messageObj);
};

// Create a Filter subscription
const { error, subscription } = await node.filter.createSubscription({ contentTopics: [contentTopic] });

if (error) {
    // handle errors if happens
    throw Error(error);
}

// Subscribe to content topics and process new messages
await subscription.subscribe([decoder], callback);
```

The `pubsubTopicShardInfo` parameter allows you to configure a different network configuration for your `Filter` subscription:

```js
// Create the network config
const networkConfig = { clusterId: 3, shards: [1, 2] };

// Create Filter subscription with custom network config
const subscription = await node.filter.createSubscription(networkConfig);
```

You can use the `subscription.unsubscribe()` function to stop receiving messages from a content topic:

```js
await subscription.unsubscribe([contentTopic]);
```

:::tip Congratulations!
You have successfully sent and received messages over the Waku Network using the `Light Push` and `Filter` protocols. Have a look at the [light-js](https://github.com/waku-org/js-waku-examples/tree/master/examples/light-js) and [light-chat](https://github.com/waku-org/js-waku-examples/tree/master/examples/light-chat) examples for working demos.
:::