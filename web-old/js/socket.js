/*
 * This file is part of Exopticon (https://github.com/dmm/exopticon).
 * Copyright (c) 2018 David Matthew Mattli
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

var loc = window.location,
  new_uri;
if (loc.protocol === "https:") {
  new_uri = "wss:";
} else {
  new_uri = "ws:";
}
new_uri += "//" + loc.host;
new_uri += loc.pathname + "v1/ws";
console.log(new_uri);

function WebSocketClient() {
  this.number = 0; // Message number
  this.autoReconnectInterval = 5 * 1000; // ms
  this.status = "enabled";
}

WebSocketClient.prototype.open = function(url) {
  this.url = url;
  this.instance = new WebSocket(this.url);
  this.instance.binaryType = "arraybuffer";
  this.instance.onopen = () => {
    this.onopen();
  };
  this.instance.onmessage = (data, flags) => {
    this.number++;
    this.onmessage(data, flags, this.number);
  };
  this.instance.onclose = e => {
    switch (e.code) {
      case 1000: // CLOSE_NORMAL
        console.log("WebSocket: closed");
        break;
      default:
        // Abnormal closure
        this.reconnect(e);
        break;
    }
    this.onclose(e);
  };

  this.instance.onerror = e => {
    switch (e.code) {
      case "ECONNREFUSED":
        this.reconnect(e);
        break;
      default:
        this.onerror(e);
        break;
    }
  };
};

WebSocketClient.prototype.send = function(data) {
  try {
    this.instance.send(data);
  } catch (e) {
    this.instance.emit("error", e);
  }
};

WebSocketClient.prototype.reconnect = function(e) {
  console.log(`WebSocketClient: retry in ${this.autoReconnectInterval}ms`, e);

  var that = this;
  setTimeout(function() {
    console.log("WebSocketClient: reconnecting...");
    if (that.status === "enabled") {
      that.open(that.url);
    }
  }, this.autoReconnectInterval);
};

WebSocketClient.prototype.onopen = function(e) {
  console.log("WebSocketClient: open", arguments);
};
WebSocketClient.prototype.onmessage = function(data, flags, number) {
  console.log("WebSocketClient: message", arguments);
};
WebSocketClient.prototype.onerror = function(e) {
  console.log("WebSocketClient: error", arguments);
};
WebSocketClient.prototype.onclose = function(e) {
  console.log("WebSocketClient: closed", arguments);
};
WebSocketClient.prototype.setStatus = function(status) {
  if (status !== "enabled" && status !== "disabled") {
    return;
  }

  this.status = status;
};

let socket = new WebSocketClient();

socket.open(new_uri);

export default socket;
