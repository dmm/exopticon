import { Injectable } from "@angular/core";
import { Duration, Instant } from "@js-joda/core";
import { ReplaySubject, BehaviorSubject, Subject } from "rxjs";
import { CameraId } from "./camera";

type ServerMessage = NegotiationAnswer;

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
  statusSubject: BehaviorSubject<ClientStatus> = new BehaviorSubject({
    kind: "paused",
  });
  private peerConnection: RTCPeerConnection;
  private dataChannel?: RTCDataChannel;
  private transceivers: Map<CameraId, RTCRtpTransceiver> = new Map();
  private emitters: Map<CameraId, ReplaySubject<MediaStream>> = new Map();
  private activeCameras: Map<CameraId, boolean> = new Map();
  private enabled: boolean = false;

  private signalSocket?: WebSocket;

  // status timer
  private retryAttempts = 0;
  private readonly baseDelay = 100;
  private readonly maxTimeout = 5000;
  private timeoutId: ReturnType<typeof setTimeout>;

  status$ = this.statusSubject.asObservable();

  constructor() {}

  //
  // public methods
  //

  enable(): void {
    this.enabled = true;
    this.updateState();
  }

  disable(): void {
    this.enabled = false;
    this.updateState();
  }

  updateActiveCameras(activeCameraIds: CameraId[]) {
    for (let [id, _val] of this.activeCameras) {
      this.activeCameras.set(id, false);
    }

    activeCameraIds.map((id) => {
      this.activeCameras.set(id, true);
    });
    this.updateState();
  }

  subscribe(cameraId: CameraId): Subject<MediaStream> {
    if (this.emitters.has(cameraId)) {
      return this.emitters.get(cameraId);
    } else {
      let ff = new ReplaySubject<MediaStream>(1);
      this.emitters.set(cameraId, ff);
      return ff;
    }
  }

  //
  // private methods
  //

  private updateStatus(newStatus: ClientStatus): void {
    this.statusSubject.next(newStatus);
  }

  /** Handles periodic state updates and transitions. */
  private updateState(): void {
    clearTimeout(this.timeoutId);

    if (!this.enabled) {
      this.disconnect("not enabled");
      return;
    }

    switch (this.statusSubject.value.kind) {
      case "paused":
        this.connect();
        break;
      case "signalChannelConnecting":
      case "webrtcConnecting":
        if (this.hasExceededTimeout(this.statusSubject.value.since)) {
          this.connect();
        }
        break;
      default:
        this.syncTracks();
    }

    this.timeoutId = setTimeout(() => this.updateState(), this.maxTimeout);
  }

  /** Checks if the WebRTC connection is in an active state. */
  private isWebrtcActive(): boolean {
    const status = this.statusSubject.value.kind;
    return status === "webrtcConnecting" || status === "webrtcConnected";
  }

  /** Checks if a timeout duration has been exceeded. */
  private hasExceededTimeout(since: Instant): boolean {
    const duration = Duration.between(since, Instant.now()).toMillis();
    return duration > this.maxTimeout;
  }

  /** Updates mappings between transceivers and cameras. */
  private updateStreamMappings(): void {
    if (this.statusSubject.value.kind === "paused" || !this.signalSocket)
      return;

    const mappings: Record<string, CameraId> = {};
    for (const [cameraId, transceiver] of this.transceivers) {
      if (transceiver.mid && this.activeCameras.get(cameraId)) {
        mappings[transceiver.mid] = cameraId;
      }
    }

    this.signalSocket.send(JSON.stringify({ kind: "streamMapping", mappings }));
  }

  /** Synchronizes transceivers with active cameras. */
  private syncTracks(): void {
    if (this.isWebrtcActive()) {
      for (const [cameraId, _] of this.activeCameras) {
        if (!this.transceivers.has(cameraId)) {
          const transceiver = this.peerConnection?.addTransceiver("video", {
            direction: "recvonly",
          });
          this.transceivers.set(cameraId, transceiver!);
        }
      }
      this.updateStreamMappings();
    } else {
      this.transceivers.clear();
    }
  }

  /** Inititates WebRTC connection */
  private webrtcConnect() {
    this.peerConnection = new RTCPeerConnection();

    this.updateStatus({ kind: "webrtcConnecting", since: Instant.now() });

    this.dataChannel = this.peerConnection.createDataChannel("foo");
    this.dataChannel.onclose = () => {};
    let self = this;
    this.dataChannel.onopen = () => {};

    this.dataChannel.onmessage = (_) => {};

    this.peerConnection.onconnectionstatechange = (ev) => {
      switch (this.peerConnection.connectionState) {
        case "new":
        case "connecting":
          this.updateStatus({ kind: "webrtcConnecting", since: Instant.now() });
          break;
        case "connected":
          this.updateStatus({ kind: "webrtcConnected", since: Instant.now() });
          self.updateState();
          break;
        case "disconnected":
          this.disconnect("webrtc disconnecting");
          break;
        case "closed":
          this.disconnect("webrtc connection offline");
          break;
        case "failed":
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
      this.sendOffer();
    };

    this.peerConnection.onicecandidate = (event) => {
      if (event.candidate !== null) {
      }
    };

    this.peerConnection.ontrack = ({ transceiver, streams: [stream] }) => {
      for (const [cameraId, tran] of this.transceivers) {
        if (tran.mid === transceiver.mid) {
          this.emitters.get(cameraId).next(stream);
        }
      }
    };
  }

  /** Send webrtc offer to server */
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

  /** Builds WebSocket url from base.href */
  private constructWebSocketUrl(): string {
    const loc = window.location;
    const basePath = document.querySelector("base")?.getAttribute("href") || "";
    const protocol = loc.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${loc.host}${basePath}v1/webrtc/connect`;
  }

  /** Initiates WebSocket connection */
  private setupSignalSocket(): void {
    const url = this.constructWebSocketUrl();
    this.signalSocket = new WebSocket(url);

    this.signalSocket.onopen = () => this.webrtcConnect();
    this.signalSocket.onclose = () => this.disconnect("signalSocket onclose");
    this.signalSocket.onerror = () => this.updateStatus({ kind: "paused" });
    this.signalSocket.onmessage = (event) => this.handleSocketMessage(event);
  }

  /** Handle messages from server over websocket */
  private handleSocketMessage(event: MessageEvent): void {
    const message: ServerMessage = JSON.parse(event.data);
    if (message.kind === "negotiationAnswer") {
      this.handleNegotiationAnswer(message);
    }
  }

  private async handleNegotiationAnswer(
    message: NegotiationAnswer,
  ): Promise<void> {
    try {
      await this.peerConnection?.setRemoteDescription({
        sdp: message.answer,
        type: "answer",
      });
    } catch (err) {
      console.error("Failed to handle negotiation answer:", err);
      this.disconnect("message error");
    }
  }

  private connect(): void {
    this.setupSignalSocket();
    this.updateStatus({
      kind: "signalChannelConnecting",
      since: Instant.now(),
    });
  }

  private disconnect(reason: string) {
    console.log("WebRTC disconnected ( " + reason + " )....!");
    this.updateStatus({ kind: "paused" });
    this.signalSocket.close();
    if (this.peerConnection) {
      this.peerConnection.close();
      this.peerConnection = undefined;
      this.dataChannel = undefined;
    }
    this.transceivers.clear();
  }
}
