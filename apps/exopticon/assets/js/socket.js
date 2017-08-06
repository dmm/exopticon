
import {Socket} from "phoenix";
import binarySocket from "./binarySocket";

/*the type=msgpack param is only added to distinguish this connection
from the phoenix live reload connection in the browser's network tab*/  
let socket = new Socket("/socket", {params: {type: "msgpack"}});

socket = binarySocket.convertToBinary(socket);

socket.connect();

export default socket;
