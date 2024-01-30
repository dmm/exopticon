import { Injectable } from "@angular/core";
import { Duration, Instant } from "@js-joda/core";
import { ReplaySubject } from "rxjs";

interface Subscription {
  id: number;
  trackId: string;
}

type ClientMessage = SubscriptionUpdate | NegotiationRequest | StreamMapping;

interface SubscriptionUpdate {
  kind: "subscriptionUpdate";
  subscribedCameraIds: number[];
}

interface NegotiationRequest {
  kind: "negotiationRequest";
  offer: string;
}

type ServerMessage = NegotiationAnswer;

interface StreamMapping {
  kind: "streamMapping";
  mappings: Map<String, number>;
}

interface NegotiationAnswer {
  kind: "negotiationAnswer";
  answer: string;
}

// ClientStatus
interface Paused {
  kind: "paused";
}

interface SignalChannelConnecting {
  kind: "signalChannelConnecting";
  since: Instant;
}

interface WebrtcConnecting {
  kind: "webrtcConnecting";
  since: Instant;
}

interface WebrtcConnected {
  kind: "webrtcConnected";
  since: Instant;
}

type ClientStatus =
  | Paused
  | SignalChannelConnecting
  | WebrtcConnecting
  | WebrtcConnected;

@Injectable({
  providedIn: "root",
})
export class WebrtcService {
  private peerConnection: RTCPeerConnection;
  private dataChannel?: RTCDataChannel;
  private subscriptions: Map<number, Subscription> = new Map();
  private transceivers: Map<number, RTCRtpTransceiver> = new Map();
  private emitters: Map<number, ReplaySubject<MediaStream>> = new Map();
  private activeCameras: Map<number, boolean> = new Map();
  private enabled: boolean = false;

  private signalSocket?: WebSocket;

  // status
  private status: ClientStatus = { kind: "paused" };

  // status timer
  private maxTimeout = 5000;
  private timeoutId: ReturnType<typeof setTimeout>;

  constructor() {}

  updateState() {
    let activeCameras = " [";
    for (let [id, val] of this.activeCameras) {
      if (val) {
        activeCameras += id + ",";
      }
    }
    activeCameras += "]";

    console.log(
      "updateState(): status: " +
        this.status.kind +
        ", Enabled? " +
        this.enabled +
        activeCameras
    );
    clearTimeout(this.timeoutId);

    if (!this.enabled) {
      this.disconnect("not enabled");
      return;
    }

    if (this.status.kind === "paused") {
      this.connect();
    } else if (this.status.kind === "signalChannelConnecting") {
      let duration = Duration.between(this.status.since, Instant.now());
      if (duration.toMillis() > this.maxTimeout) {
        this.disconnect("signal Channel Connecting timeout");
      }
    } else if (this.status.kind === "webrtcConnecting") {
      let duration = Duration.between(this.status.since, Instant.now());
      if (duration.toMillis() > this.maxTimeout) {
        this.sendOffer();
      }
    }

    let s = "closed";
    if (this.peerConnection) {
      s = this.peerConnection.connectionState;
    }
    console.log("WEBRTC STATUS: " + s);
    if (s === "new") {
    } else if (s === "connecting") {
      this.syncTracks();
      this.updateSubscriptions();
    } else if (s === "connected") {
      this.syncTracks();
      this.updateSubscriptions();
    } else if (s === "disconnected") {
    } else if (s === "failed") {
    } else if (s === "closed") {
    }

    if (this.enabled === true) {
      this.timeoutId = setTimeout(this.updateState.bind(this), this.maxTimeout);
    }
  }

  syncTracks() {
    if (
      this.status.kind === "webrtcConnecting" ||
      this.status.kind == "webrtcConnected"
    ) {
      // ensure every cameraId has a corresponding transceiver
      for (let [cameraId, cameraActive] of this.activeCameras) {
        let direction: RTCRtpTransceiverDirection = cameraActive
          ? "recvonly"
          : "inactive";
        let transceiver = this.transceivers.get(cameraId);
        if (transceiver === undefined) {
          console.log("CREATING tranceiver for " + cameraId);
          let newTransceiver = this.peerConnection.addTransceiver("video", {
            direction: direction,
          });
          this.transceivers.set(cameraId, newTransceiver);
        }
      }
      this.updateMappings();
    } else {
      // clean up transceivers
      this.transceivers.clear();
    }
  }

  enable() {
    this.enabled = true;
    this.updateState();
  }

  disable() {
    this.enabled = false;
    this.updateState();
  }

  updateActiveCameras(activeCameraIds: number[]) {
    for (let [id, _val] of this.activeCameras) {
      this.activeCameras.set(id, false);
    }

    activeCameraIds.map((id) => {
      this.activeCameras.set(id, true);
    });
    this.updateState();
  }

  subscribe(cameraId: number): ReplaySubject<MediaStream> {
    if (this.emitters.has(cameraId)) {
      return this.emitters.get(cameraId);
    } else {
      let ff = new ReplaySubject<MediaStream>(1);
      this.emitters.set(cameraId, ff);
      return ff;
    }
  }

  private updateMappings() {
    if (
      this.status.kind === "paused" ||
      this.status.kind === "signalChannelConnecting"
    ) {
      return;
    }

    let mappings = new Object();

    for (let [cameraId, transceiver] of this.transceivers) {
      if (transceiver.mid !== null) {
        mappings[transceiver.mid] = cameraId;
      }
    }
    this.signalSocket.send(
      JSON.stringify({
        kind: "streamMapping",
        mappings: mappings,
      })
    );
  }

  private updateSubscriptions() {
    for (const [cameraId, transceiver] of this.transceivers) {
      const newDirection = this.activeCameras.get(cameraId)
        ? "recvonly"
        : "inactive";

      if (transceiver.currentDirection === "stopped") {
        this.transceivers.delete(cameraId);
        return;
      }

      if (transceiver.direction !== newDirection) {
        console.log(
          "Updating " +
            cameraId +
            " from " +
            transceiver.direction +
            " to " +
            newDirection
        );
        transceiver.direction = newDirection;
      }
    }
  }

  private webrtcConnect() {
    this.peerConnection = new RTCPeerConnection();

    this.status = { kind: "webrtcConnecting", since: Instant.now() };

    this.dataChannel = this.peerConnection.createDataChannel("foo");
    this.dataChannel.onclose = () => console.log("sendChannel has closed");
    let self = this;
    this.dataChannel.onopen = () => {
      console.log("sendChannel has opened");
    };
    this.dataChannel.onmessage = (e) => {};

    this.peerConnection.onconnectionstatechange = (ev) => {
      switch (this.peerConnection.connectionState) {
        case "new":
        case "connecting":
          console.log("WebRTC Connecting...");
          this.status = { kind: "webrtcConnecting", since: Instant.now() };
          break;
        case "connected":
          console.log("Online **");
          this.status = { kind: "webrtcConnected", since: Instant.now() };
          self.updateState();
          break;
        case "disconnected":
          console.log("Disconnecting...");
          this.disconnect("webrtc disconnecting");
          break;
        case "closed":
          console.log("Connection Offline");
          this.disconnect("webrtc connection offline");
          break;
        case "failed":
          console.log("Connection Error");
          this.disconnect("webrtc failed");
          break;
        default:
          console.log("Unknown");
          break;
      }
    };

    this.peerConnection.oniceconnectionstatechange = (e) => {
      let state = this.peerConnection.iceConnectionState;
      if (state === "connected") {
      } else if (state === "disconnected" || state === "failed") {
      }
      this.updateState();
    };

    this.peerConnection.onnegotiationneeded = async (e) => {
      console.log("Renegotiation requested... Creating offer!");
      this.sendOffer();
    };

    this.peerConnection.onicecandidate = (event) => {
      if (event.candidate !== null) {
      }
    };

    this.peerConnection.ontrack = ({ transceiver, streams: [stream] }) => {
      console.log("Got track! " + transceiver.mid);
      for (const [cameraId, tran] of this.transceivers) {
        if (tran.mid === transceiver.mid) {
          this.emitters.get(cameraId).next(stream);
        }
      }
    };
  }

  private async sendOffer() {
    try {
      let offer = await this.peerConnection.createOffer();

      await this.peerConnection.setLocalDescription(offer);

      let offerMsg = {
        kind: "negotiationRequest",
        offer: this.peerConnection.localDescription.sdp,
      };
      let offer_string = JSON.stringify(offerMsg);
      this.signalSocket.send(offer_string);
    } catch (error) {
      console.log("Error sending offer! " + error);
    }
  }

  private connect() {
    let url = "";
    let parse = document.createElement("a");
    parse.href = document.querySelector("base")["href"];

    let loc = window.location;
    if (loc.protocol === "https:") {
      url = "wss:";
    } else {
      url = "ws:";
    }
    var pathname = parse.pathname === "/" ? "" : `/${parse.pathname}`;
    url += `//${parse.host}${pathname}/v1/webrtc/connect`;

    this.signalSocket = new WebSocket(url);

    this.status = {
      kind: "signalChannelConnecting",
      since: Instant.now(),
    };

    this.signalSocket.onopen = (event) => {
      this.webrtcConnect();
    };

    this.signalSocket.onclose = (event) => {
      this.disconnect("signalSocket onclose");
    };

    this.signalSocket.onerror = (event) => {
      this.status = { kind: "paused" };
    };

    this.signalSocket.onmessage = async (event) => {
      let message: ServerMessage = JSON.parse(event.data);

      try {
        switch (message.kind) {
          case "negotiationAnswer":
            console.log("GOT ANSWER........");
            await this.peerConnection.setRemoteDescription({
              sdp: message.answer,
              type: "answer",
            });
            break;
        }
      } catch (err) {
        console.log("Message error! " + err);
        this.disconnect("message error");
      }
    };
  }

  private disconnect(reason) {
    console.log("disconnect( " + reason + " )....!");
    this.status = { kind: "paused" };
    this.signalSocket.close();
    if (this.peerConnection) {
      this.peerConnection.close();
      this.peerConnection = undefined;
      this.dataChannel = undefined;
    }
    this.subscriptions.clear();
    this.transceivers.clear();
  }
}
