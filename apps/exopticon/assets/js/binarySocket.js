import msgpack from './msgpack';

/**
 * convert a phoenix socket to a msgpacked binary version
 * @param {Object} socket - phoenix socket to convert
 * @return {Object} a msgpacked binary socket
 */
function convertToBinary(socket) {
  let parentOnConnOpen = socket.onConnOpen;

  socket.onConnOpen = function(...args) {
    // setting this to arraybuffer will help us not having to deal
    // with blobs
    this.conn.binaryType = 'arraybuffer';
    parentOnConnOpen.apply(this, args);
  };

  // we also need to override the onConnMessage function, where we'll
  // be checking for binary data, and delegate to the default
  // implementation if it's not what we expected
  let parentOnConnMessage = socket.onConnMessage;

  socket.onConnMessage = function(rawMessage, ...args) {
    if (!(rawMessage.data instanceof window.ArrayBuffer)) {
      return parentOnConnMessage.apply(this, args);
    }
    let msg = decodeMessage(rawMessage.data);
    let topic = msg.topic;
    let event = msg.event;
    let payload = msg.payload;
    let ref = msg.ref;

    this.log('receive', (payload.status || '') + ' ' + topic + ' ' + event
             + ' ' + (ref && '(' + ref + ')' || ''), payload);

    // The default implementation of onConnMessage does this to reset
    // the heartbeat timeout.  Duplicate this because we are never
    // calling the default implementation, for now.
    if (ref && ref === this.pendingHeartbeatRef) {
 this.pendingHeartbeatRef = null;
}

    this.channels.filter(function(channel) {
      return channel.isMember(topic);
    }).forEach(function(channel) {
      return channel.trigger(event, payload, ref);
    });
    this.stateChangeCallbacks.message.forEach(function(callback) {
      return callback(msg);
    });
  };

  return socket;
}
/**
 * @param {ArrayBuffer} rawdata
 * @return {Object} decoded object
 */
function decodeMessage(rawdata) {
    if (!rawdata) {
        return undefined;
    }
    let binary = new Uint8Array(rawdata);
    let data;
    data = binary;

    let msg = msgpack.decode(data);
    return msg;
}

export default {
  convertToBinary,
};
