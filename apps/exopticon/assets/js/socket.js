
import {Socket} from "phoenix";
import binarySocket from "./binarySocket";

/*the type=msgpack param is only added to distinguish this connection
from the phoenix live reload connection in the browser's network tab*/  
let socket = new Socket("/socket", {params: {type: "msgpack"}});

socket = binarySocket.convertToBinary(socket);

socket.connect();

//lets join the lobby
let channel = socket.channel("test:lobby", {});

channel.on("small_reply", function(data) {
  console.log("small reply: server responded with", data);
});

channel.on("large_reply", function(data) {
  console.log("large reply: server responded with", data);
});

function renderFrame(img, imageArrayBuffer) {
    var blob  = new Blob([imageArrayBuffer],{type: "image/jpeg"});
    img.onload = function (e) {
        window.URL.revokeObjectURL(img.src);
        img = null;
    };

    img.onerror = img.onabort = function () {
        console.log('error loading image!');
        img = null;
    };
    img.src = window.URL.createObjectURL(blob);
}

channel.on("jpg", function(data) {
    console.log("Received frame!");
    renderFrame(document.getElementById("video"), data.frameJpeg);
});

channel.join()
  .receive("ok", resp => {
    console.log("Joined successfully", resp);
    channel.push("small");
    channel.push("large");
  })
  .receive("error", resp => { console.log("Unable to join", resp) });

export default socket;
