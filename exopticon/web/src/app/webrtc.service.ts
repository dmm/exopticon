import { Injectable } from "@angular/core";
import { ReplaySubject } from "rxjs";

interface Offer {
  kind: "offer";
  sdp: string;
}

interface Answer {
  kind: "answer";
  sdp: string;
}

interface Candidate {
  kind: "candidate";
  candidate: RTCIceCandidateInit;
}

interface UpdateSubscriptions {
  kind: "updateSubscriptions";
  subscriptions: {
    cameraId: number;
    trackId: string;
  }[];
}

interface CameraStatus {
  kind: "cameraStatus";
  cameraId: number;
  status: boolean;
}
type SignalMessage =
  | Offer
  | Answer
  | Candidate
  | UpdateSubscriptions
  | CameraStatus;

interface Subscription {
  id: number;
  trackId: string;
}

enum Status {
  Paused,
  Connecting,
  Connected,
}

@Injectable({
  providedIn: "root",
})
export class WebrtcService {
  private peerConnection: RTCPeerConnection;
  private dataChannel?: RTCDataChannel;
  private signalSocket?: WebSocket;
  private subscriptions: Map<number, Subscription> = new Map();
  private candidates: RTCIceCandidateInit[] = new Array();
  private emitters: Map<number, ReplaySubject<MediaStream>> = new Map();
  private activeCameras: Map<number, boolean> = new Map();
  private enabled: boolean = false;

  // status
  private signalSocketStatus: Status = Status.Paused;
  private webrtcStatus: Status = Status.Paused;

  // status timer
  private minTimeout = 500;
  private maxTimeout = 5000;
  private timeoutId: ReturnType<typeof setTimeout>;

  contructor() {}

  updateState() {
    clearTimeout(this.timeoutId);

    console.dir(`Active cameras: ${this.activeCameras.entries()}`);
    // if enabled but disconnected...
    if (this.enabled && this.signalSocketStatus === Status.Paused) {
      this.connect();
    } else if (!this.enabled && this.signalSocketStatus !== Status.Paused) {
      this.disconnect();
    } else if (
      this.enabled &&
      this.signalSocketStatus === Status.Connected &&
      this.webrtcStatus === Status.Paused
    ) {
      this.webrtcConnect();
    } else if (
      this.enabled &&
      this.signalSocketStatus == Status.Connected &&
      this.webrtcStatus == Status.Connected
    ) {
      this.updateSubscriptions();
    }

    if (this.enabled === true) {
      this.timeoutId = setTimeout(this.updateState.bind(this), this.maxTimeout);
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

  private addTrack(cameraId: number, trackId: string) {
    this.subscriptions.set(cameraId, {
      id: cameraId,
      trackId: trackId,
    });
    this.peerConnection.addTransceiver("video", {
      direction: "recvonly",
    });
    if (!this.emitters.has(cameraId)) {
      this.emitters.set(cameraId, new ReplaySubject<MediaStream>(1));
    }
  }

  private updateSubscriptions() {
    if (
      this.signalSocket === undefined ||
      this.signalSocket.readyState !== this.signalSocket.OPEN
    ) {
      return;
    }

    for (const sub of this.subscriptions.values()) {
      let status = false;
      if (this.activeCameras.has(sub.id)) {
        status = this.activeCameras.get(sub.id);
      }
      this.signalSocket.send(
        JSON.stringify({
          kind: "cameraStatus",
          id: sub.id,
          status: status,
        })
      );
    }
  }

  private webrtcConnect() {
    this.peerConnection = new RTCPeerConnection();
    this.webrtcStatus = Status.Connecting;

    this.peerConnection.onconnectionstatechange = (ev) => {
      switch (this.peerConnection.connectionState) {
        case "new":
        case "connecting":
          console.log("Connecting...");
          break;
        case "connected":
          console.log("Online");
          break;
        case "disconnected":
          console.log("Disconnecting...");
          break;
        case "closed":
          console.log("Offline");
          break;
        case "failed":
          console.log("Error");
          break;
        default:
          console.log("Unknown");
          break;
      }
    };

    this.peerConnection.oniceconnectionstatechange = (e) => {
      let state = this.peerConnection.iceConnectionState;
      console.log("ICE CONNECTION STATE: " + state);
      if (state === "connected") {
        this.webrtcStatus = Status.Connected;
      }
      this.updateState();
    };

    this.peerConnection.onnegotiationneeded = async (e) => {
      console.log("Renegotiation requested... Creating offer!");
      await this.sendOffer();
    };

    //    this.peerConnection.onicecandidate = (event) => {};

    this.peerConnection.ontrack = (event) => {
      for (let sub of this.subscriptions.values()) {
        if (sub.trackId === event.streams[0].id) {
          console.log("EMITTING TRACK! " + sub.trackId);
          this.emitters.get(sub.id).next(event.streams[0]);
        }
      }
    };
  }

  private async sendOffer() {
    console.log("SENDING OFFER!");
    try {
      let offer = await this.peerConnection.createOffer();
      await this.peerConnection.setLocalDescription(offer);
      this.signalSocket.send(
        JSON.stringify({
          kind: "offer",
          sdp: offer.sdp,
        })
      );
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
    url += `//${parse.host}${pathname}/v1/ws`;
    console.log(`websocket url: ${url}`);

    this.webrtcConnect();
    this.signalSocket = new WebSocket(url);

    this.signalSocketStatus = Status.Connecting;

    this.signalSocket.onopen = (event) => {
      this.signalSocketStatus = Status.Connected;
      this.updateSubscriptions();
      this.dataChannel = this.peerConnection.createDataChannel("foo");
      this.dataChannel.onclose = () => console.log("sendChannel has closed");
      this.dataChannel.onopen = () => console.log("sendChannel has opened");
      this.dataChannel.onmessage = (e) =>
        console.log(
          `Message from DataChannel '${this.dataChannel.label}' payload '${e.data}'`
        );
    };

    this.signalSocket.onclose = (event) => {
      this.disconnect();
    };

    this.signalSocket.onerror = (event) => {
      this.signalSocketStatus = Status.Paused;
    };

    this.signalSocket.onmessage = async (event) => {
      let message: SignalMessage = JSON.parse(event.data);

      try {
        switch (message.kind) {
          case "offer":
            break;
          case "answer":
            console.log(
              "GOT ANSWER! " + this.peerConnection.iceConnectionState
            );
            //            console.log("SDP: " + message.sdp);

            await this.peerConnection.setRemoteDescription({
              sdp: message.sdp,
              type: "answer",
            });
            for (const c of this.candidates) {
              await this.peerConnection.addIceCandidate(c);
            }
            this.candidates = new Array();
            break;
          case "candidate":
            console.log("Got candidate! " + message.candidate.candidate);
            if (this.peerConnection.currentRemoteDescription) {
              let res = await this.peerConnection.addIceCandidate(
                message.candidate
              );
              console.log("Add candidate result " + res);
            } else {
              this.candidates.push(message.candidate);
            }
            break;
          case "updateSubscriptions":
            console.log("Got new track subscriptions!");
            message.subscriptions.forEach((s) => {
              if (!this.subscriptions.has(s.cameraId)) {
                this.addTrack(s.cameraId, s.trackId);
              }
            });
        }
      } catch (err) {
        console.log("Message error! " + err);
        this.disconnect();
        setTimeout(() => {
          this.connect();
        }, 1000);
      }
    };
  }

  private disconnect() {
    this.signalSocket.close();
    this.peerConnection.close();
    this.subscriptions.clear();
    this.signalSocketStatus = Status.Paused;
  }
}
