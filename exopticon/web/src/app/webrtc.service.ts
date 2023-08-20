import { HttpClient } from "@angular/common/http";
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
  private webrtcStatus: Status = Status.Paused;

  // status timer
  private minTimeout = 500;
  private maxTimeout = 5000;
  private timeoutId: ReturnType<typeof setTimeout>;

  constructor(private http: HttpClient) {}

  updateState() {
    clearTimeout(this.timeoutId);

    // if enabled but disconnected...
    if (this.enabled && this.webrtcStatus === Status.Paused) {
      this.connect();
    } else if (!this.enabled && this.webrtcStatus !== Status.Paused) {
      this.disconnect();
    } else if (this.enabled && this.webrtcStatus == Status.Connected) {
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
    // if (
    //   this.signalSocket === undefined ||
    //   this.signalSocket.readyState !== this.signalSocket.OPEN
    // ) {
    //   return;
    // }
    // for (const sub of this.subscriptions.values()) {
    //   let status = false;
    //   if (this.activeCameras.has(sub.id)) {
    //     status = this.activeCameras.get(sub.id);
    //   }
    //   this.signalSocket.send(
    //     JSON.stringify({
    //       kind: "cameraStatus",
    //       id: sub.id,
    //       status: status,
    //     })
    //   );
    // }
  }

  private webrtcConnect() {
    this.peerConnection = new RTCPeerConnection();
    this.webrtcStatus = Status.Connecting;

    this.dataChannel = this.peerConnection.createDataChannel("foo");
    this.dataChannel.onclose = () => console.log("sendChannel has closed");
    this.dataChannel.onopen = () => console.log("sendChannel has opened");
    this.dataChannel.onmessage = (e) => {};

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
          console.log("Connection Offline");
          break;
        case "failed":
          console.log("Connection Error");
          break;
        default:
          console.log("Unknown");
          break;
      }
    };

    this.peerConnection.oniceconnectionstatechange = (e) => {
      let state = this.peerConnection.iceConnectionState;
      if (state === "connected") {
        this.webrtcStatus = Status.Connected;
      } else if (state === "disconnected" || state === "failed") {
        this.webrtcStatus = Status.Paused;
      }
      this.updateState();
    };

    this.peerConnection.onnegotiationneeded = async (e) => {
      console.log("Renegotiation requested... Creating offer!");
      this.negotiating = true;
      this.sendOffer();
    };

    this.peerConnection.onicecandidate = (event) => {
      if (event.candidate !== null) {
      }
    };

    this.peerConnection.ontrack = (event) => {
      for (let sub of this.subscriptions.values()) {
        if (sub.trackId === event.streams[0].id) {
          this.emitters.get(sub.id).next(event.streams[0]);
        }
      }
    };
  }

  private async sendOffer() {
    try {
      let offer = await this.peerConnection.createOffer();

      await this.peerConnection.setLocalDescription(offer);

      let answer = await this.http
        .post<RTCSessionDescriptionInit>("/v1/webrtc/connect", offer)
        .toPromise();

      await this.peerConnection.setRemoteDescription(answer);
      this.negotiating = false;
    } catch (error) {
      console.log("Error sending offer! " + error);
      this.negotiating = false;
    }
  }

  private connect() {
    this.webrtcConnect();
    // this.signalSocket = new WebSocket(url);

    // this.signalSocketStatus = Status.Connecting;

    // this.signalSocket.onopen = (event) => {
    //   this.signalSocketStatus = Status.Connected;
    //   return;
    //   this.dataChannel = this.peerConnection.createDataChannel("foo");
    //   this.dataChannel.onclose = () => console.log("sendChannel has closed");
    //   this.dataChannel.onopen = () => console.log("sendChannel has opened");
    //   this.dataChannel.onmessage = (e) => {};
    // };

    // this.signalSocket.onclose = (event) => {
    //   this.disconnect();
    // };

    // this.signalSocket.onerror = (event) => {
    //   this.signalSocketStatus = Status.Paused;
    // };

    // this.signalSocket.onmessage = async (event) => {
    //   let message: SignalMessage = JSON.parse(event.data);

    //   try {
    //     switch (message.kind) {
    //       case "offer":
    //         break;
    //       case "answer":
    //         await this.peerConnection.setRemoteDescription({
    //           sdp: message.sdp,
    //           type: "answer",
    //         });
    //         for (const c of this.candidates) {
    //           await this.peerConnection.addIceCandidate(c);
    //         }
    //         this.candidates = new Array();
    //         break;
    //       case "candidate":
    //         if (this.peerConnection.currentRemoteDescription) {
    //           let res = await this.peerConnection.addIceCandidate(
    //             message.candidate
    //           );
    //         } else {
    //           this.candidates.push(message.candidate);
    //         }
    //         break;
    //       case "updateSubscriptions":
    //         message.subscriptions.forEach((s) => {
    //           if (!this.subscriptions.has(s.cameraId)) {
    //             this.addTrack(s.cameraId, s.trackId);
    //           }
    //         });
    //     }
    //   } catch (err) {
    //     console.log("Message error! " + err);
    //     this.disconnect();
    //     setTimeout(() => {
    //       this.connect();
    //     }, 1000);
    //   }
    // };
  }

  private disconnect() {
    //    this.signalSocket.close();
    this.peerConnection.close();
    this.subscriptions.clear();
  }
}
