import { Injectable } from "@angular/core";
import { Duration, Instant } from "@js-joda/core";
import { ReplaySubject, BehaviorSubject, Subject } from "rxjs";
import { CameraId } from "./camera";

type ServerMessage = NegotiationAnswer;

interface NegotiationAnswer {
  kind: "negotiationAnswer";
  answer: string;
}

type TimeoutId = ReturnType<typeof setTimeout>;

// Events
type WebRtcEvent =
  | { type: "ENABLE" }
  | { type: "DISABLE" }
  | { type: "WEBSOCKET_OPEN" }
  | { type: "WEBSOCKET_CLOSE"; reason: string }
  | { type: "WEBSOCKET_ERROR" }
  | { type: "WEBRTC_CONNECTING" }
  | { type: "WEBRTC_CONNECTED" }
  | { type: "WEBRTC_DISCONNECTED" }
  | { type: "WEBRTC_FAILED" }
  | { type: "NEGOTIATION_NEEDED" }
  | { type: "NEGOTIATION_FAILED" }
  | { type: "NEGOTIATION_ANSWER"; answer: string }
  | {
      type: "TRACK_RECEIVED";
      transceiver: RTCRtpTransceiver;
      stream: MediaStream;
    }
  | { type: "TIMEOUT" }
  | { type: "UPDATE_CAMERAS"; cameras: CameraId[] };

// States
type State =
  | { kind: "disabled" }
  | {
      kind: "connecting_signal";
      socket: WebSocket;
      timeoutId: TimeoutId;
    }
  | {
      kind: "connecting_webrtc";
      socket: WebSocket;
      pc: RTCPeerConnection;
      timeoutId: TimeoutId;
    }
  | { kind: "connected"; socket: WebSocket; pc: RTCPeerConnection }
  | { kind: "reconnecting"; attempt: number; timeoutId: TimeoutId };

interface TransceiverPair {
  video: RTCRtpTransceiver;
  audio: RTCRtpTransceiver;
}

interface MidPair {
  video: string;
  audio: string;
}

@Injectable({
  providedIn: "root",
})
export class WebrtcService {
  private state: State = { kind: "disabled" };
  private eventQueue: WebRtcEvent[] = [];
  private processing = false;

  private transceivers: Map<CameraId, TransceiverPair> = new Map();
  private emitters: Map<CameraId, ReplaySubject<MediaStream>> = new Map();
  private activeCameras: Map<CameraId, boolean> = new Map();

  statusSubject: BehaviorSubject<State> = new BehaviorSubject(this.state);

  // status timer
  private readonly maxTimeout = 5000;

  status$ = this.statusSubject.asObservable();

  constructor() {}

  //
  // public methods
  //

  enable(): void {
    this.enqueueEvent({ type: "ENABLE" });
  }

  disable(): void {
    this.enqueueEvent({ type: "DISABLE" });
  }

  updateActiveCameras(activeCameraIds: CameraId[]) {
    this.enqueueEvent({ type: "UPDATE_CAMERAS", cameras: activeCameraIds });
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

  private getActiveCameras(): CameraId[] {
    return [...this.activeCameras].filter(([_, v]) => v).map(([k]) => k);
  }

  private enqueueEvent(event: WebRtcEvent): void {
    this.eventQueue.push(event);
    this.processEvents();
  }

  private async processEvents(): Promise<void> {
    if (this.processing) return;
    this.processing = true;

    try {
      while (this.eventQueue.length > 0) {
        const event = this.eventQueue.shift()!;
        await this.handleEvent(event);
      }
    } finally {
      this.processing = false;
    }
  }

  private async handleEvent(event: WebRtcEvent): Promise<void> {
    console.log(`[${this.state.kind}] Processing event:`, event.type);

    const prevState = this.state.kind;

    switch (this.state.kind) {
      case "disabled":
        this.state = this.handleDisabledState(event);
        break;
      case "connecting_signal":
        this.state = await this.handleConnectingSignalState(event);
        break;
      case "connecting_webrtc":
        this.state = await this.handleConnectingWebrtcState(event);
        break;
      case "connected":
        this.state = await this.handleConnectedState(event);
        break;
      case "reconnecting":
        this.state = await this.handleReconnectingState(event);
        break;
    }

    if (prevState !== this.state.kind) {
      console.log(`State transition: ${prevState} -> ${this.state.kind}`);
      this.statusSubject.next(this.state);
    }
  }

  // State handlers
  //
  // All manipulation of state (RTCPeerConnection, WebSocket, etc) must occur
  // with these state handlers. Callbacks must only enqueue events. This ensures
  // we are always acting on the latest version of objects, and not state
  // references.

  private handleDisabledState(event: WebRtcEvent): State {
    if (this.state.kind !== "disabled") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "ENABLE":
        return {
          kind: "connecting_signal",
          socket: this.setupSignalSocket(),
          timeoutId: setTimeout(() => {
            this.enqueueEvent({ type: "TIMEOUT" });
          }, this.maxTimeout),
        };
      case "DISABLE":
        return this.state;
      default:
        console.warn(`Ignoring event ${event.type}, in disabled state.`);
        return this.state;
    }
  }

  private handleConnectingSignalState(event: WebRtcEvent): State {
    if (this.state.kind !== "connecting_signal") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "DISABLE":
        this.cleanupTimeout(this.state.timeoutId);
        this.cleanup(this.state.socket, null);
        return { kind: "disabled" };

      case "UPDATE_CAMERAS":
        console.log(`ACTIVE CAMERAS: ${event.cameras}`);
        this.syncTracks(event.cameras);
        return this.state;

      case "WEBSOCKET_OPEN":
        this.cleanupTimeout(this.state.timeoutId);
        const pc = this.webrtcConnect(this.state.socket);
        const active = this.getActiveCameras();
        this.syncTracks(active, pc);
        return {
          kind: "connecting_webrtc",
          socket: this.state.socket,
          pc: pc,
          timeoutId: setTimeout(() => {
            this.enqueueEvent({ type: "TIMEOUT" });
          }, this.maxTimeout),
        };

      case "WEBSOCKET_CLOSE":
      case "WEBSOCKET_ERROR":
        this.cleanupTimeout(this.state.timeoutId);
        return this.generateReconnectState(1);

      case "TIMEOUT":
        console.warn("Signal connection timeout");
        this.cleanup(this.state.socket, null);
        return this.generateReconnectState(1);

      default:
        return this.state;
    }
  }

  private handleConnectingWebrtcState(event: WebRtcEvent): State {
    if (this.state.kind !== "connecting_webrtc") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "DISABLE":
        this.cleanupTimeout(this.state.timeoutId);
        this.cleanup(this.state.socket, this.state.pc);
        return { kind: "disabled" };

      case "UPDATE_CAMERAS":
        console.log(`ACTIVE CAMERAS: ${event.cameras}`);
        this.syncTracks(event.cameras, this.state.pc);
        this.updateStreamMappings(this.state.socket);
        return this.state;

      case "WEBRTC_CONNECTED":
        this.cleanupTimeout(this.state.timeoutId);
        const active = this.getActiveCameras();
        this.syncTracks(active, this.state.pc);
        this.updateStreamMappings(this.state.socket);

        return {
          kind: "connected",
          socket: this.state.socket,
          pc: this.state.pc,
        };

      case "WEBRTC_FAILED":
      case "WEBSOCKET_ERROR":
      case "WEBSOCKET_CLOSE":
        this.cleanupTimeout(this.state.timeoutId);
        this.cleanup(this.state.socket, this.state.pc);
        return this.generateReconnectState(1);

      case "TIMEOUT":
        console.warn("WebRTC connection timeout");
        this.cleanup(this.state.socket, this.state.pc);
        return this.generateReconnectState(1);

      case "NEGOTIATION_NEEDED":
        this.sendOffer(this.state.socket, this.state.pc);
        return this.state;

      case "NEGOTIATION_ANSWER":
        this.handleNegotiationAnswer(this.state.pc, event.answer);
        return this.state;

      default:
        return this.state;
    }
  }

  private handleConnectedState(event: WebRtcEvent): State {
    if (this.state.kind !== "connected") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "DISABLE":
        this.cleanup(this.state.socket, this.state.pc);
        return { kind: "disabled" };

      case "UPDATE_CAMERAS":
        console.log(`ACTIVE CAMERAS: ${event.cameras}`);
        this.syncTracks(event.cameras, this.state.pc);
        this.updateStreamMappings(this.state.socket);
        return this.state;

      case "WEBRTC_CONNECTED":
        // We got a WEBRTC_CONNECTED when we're already in the
        // connected state. This happens when we renegotiate another
        // stream, so update the stream mappings.
        this.updateStreamMappings(this.state.socket);
        return this.state;

      case "WEBRTC_DISCONNECTED":
      case "WEBRTC_FAILED":
      case "WEBSOCKET_CLOSE":
      case "NEGOTIATION_FAILED":
        this.cleanup(this.state.socket, this.state.pc);
        return this.generateReconnectState(1);

      case "NEGOTIATION_NEEDED":
        this.sendOffer(this.state.socket, this.state.pc);
        return this.state;

      case "NEGOTIATION_ANSWER":
        this.handleNegotiationAnswer(this.state.pc, event.answer);
        this.updateStreamMappings(this.state.socket);
        return this.state;

      default:
        return this.state;
    }
  }

  private handleReconnectingState(event: WebRtcEvent): State {
    if (this.state.kind !== "reconnecting") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "DISABLE":
        this.cleanupTimeout(this.state.timeoutId);
        return { kind: "disabled" };

      case "UPDATE_CAMERAS":
        console.log(`ACTIVE CAMERAS: ${event.cameras}`);
        this.syncTracks(event.cameras);
        return this.state;

      case "TIMEOUT":
        // Retry logic here
        return {
          kind: "connecting_signal",
          socket: this.setupSignalSocket(),
          timeoutId: setTimeout(() => {
            this.enqueueEvent({ type: "TIMEOUT" });
          }, this.maxTimeout),
        };

      default:
        return this.state;
    }
  }

  private generateReconnectState(attempt: number): State {
    const delay = Math.min(1000 * Math.pow(2, attempt - 1), 30000);
    const reconnectTimeoutId = setTimeout(() => {
      this.enqueueEvent({ type: "TIMEOUT" });
    }, delay);

    return {
      kind: "reconnecting",
      attempt: attempt,
      timeoutId: reconnectTimeoutId,
    };
  }

  private setupSignalSocket(): WebSocket {
    const url = this.constructWebSocketUrl();
    let signalSocket = new WebSocket(url);

    // WebSocket callbacks must only enqueue events, not carry references which
    // may become stale.
    signalSocket.onopen = () => this.enqueueEvent({ type: "WEBSOCKET_OPEN" });
    signalSocket.onclose = () =>
      this.enqueueEvent({ type: "WEBSOCKET_CLOSE", reason: "closed" });
    signalSocket.onerror = () => this.enqueueEvent({ type: "WEBSOCKET_ERROR" });
    signalSocket.onmessage = (event) => this.handleSocketMessage(event);

    return signalSocket;
  }

  private hasExceededTimeout(since: Instant): boolean {
    const duration = Duration.between(since, Instant.now()).toMillis();
    return duration > this.maxTimeout;
  }

  // Updates mappings between transceivers and *active* cameras
  private updateStreamMappings(socket: WebSocket): void {
    const mappings: Record<CameraId, MidPair> = {};
    for (const [cameraId, tPair] of this.transceivers) {
      if (
        tPair.video.mid &&
        tPair.audio.mid &&
        this.activeCameras.get(cameraId)
      ) {
        mappings[cameraId] = {
          video: tPair.video.mid,
          audio: tPair.audio.mid,
        };
      }
    }

    socket.send(JSON.stringify({ kind: "streamMapping", mappings }));
  }

  // Synchronizes transceivers with active cameras
  private syncTracks(activeCameras: CameraId[], pc?: RTCPeerConnection): void {
    for (let [id, _val] of this.activeCameras) {
      this.activeCameras.set(id, false);
    }

    for (const cameraId of activeCameras) {
      this.activeCameras.set(cameraId, true);
      if (!this.transceivers.has(cameraId) && pc) {
        const videoTransceiver = pc?.addTransceiver("video", {
          direction: "recvonly",
        });
        const audioTransceiver = pc?.addTransceiver("audio", {
          direction: "recvonly",
        });

        this.transceivers.set(cameraId, {
          video: videoTransceiver,
          audio: audioTransceiver,
        });
      }
    }
  }

  // Inititates WebRTC connection
  private webrtcConnect(socket: WebSocket): RTCPeerConnection {
    let peerConnection = new RTCPeerConnection();

    let dataChannel = peerConnection.createDataChannel("foo");
    dataChannel.onclose = () => {};
    dataChannel.onopen = () => {};

    dataChannel.onmessage = (_) => {};

    //
    // RTCPeerConnection callbacks must only enqueue events, not use references
    // which might become stale.
    //
    peerConnection.onconnectionstatechange = () => {
      console.log(`CONNECTION STATE CHANGE: ${peerConnection.connectionState}`);
      switch (peerConnection?.connectionState) {
        case "connected":
          this.enqueueEvent({ type: "WEBRTC_CONNECTED" });
          break;
        case "disconnected":
        case "closed":
          this.enqueueEvent({ type: "WEBRTC_DISCONNECTED" });
          break;
        case "failed":
          this.enqueueEvent({ type: "WEBRTC_FAILED" });
          break;
      }
    };

    peerConnection.oniceconnectionstatechange = (e) => {
      console.log(
        `ICE CONNECTION STATE CHANGE: ${peerConnection.iceConnectionState}`,
      );

      let state = peerConnection.iceConnectionState;
      if (state === "connected") {
      } else if (state === "disconnected" || state === "failed") {
      }
      //      this.updateState();
    };

    peerConnection.onnegotiationneeded = async (_e) => {
      this.enqueueEvent({ type: "NEGOTIATION_NEEDED" });
    };

    peerConnection.onicecandidate = (event) => {
      if (event.candidate !== null) {
      }
    };

    peerConnection.ontrack = ({
      transceiver: newTransceiver,
      streams: [stream],
    }) => {
      for (const [cameraId, tPair] of this.transceivers) {
        if (
          tPair.video.mid === newTransceiver.mid ||
          tPair.audio.mid === newTransceiver.mid
        ) {
          console.log(`FETCHING EMITTER FOR CAMERA ID: ${cameraId}`);
          let tracks = new Array();
          if (tPair.video.receiver.track) {
            tracks.push(tPair.video.receiver.track);
          }
          if (tPair.audio.receiver.track) {
            tracks.push(tPair.audio.receiver.track);
          }

          MediaStream;
          const joinedStream = new MediaStream(tracks);
          this.emitters.get(cameraId).next(joinedStream);
        }
      }
    };

    return peerConnection;
  }

  // Send webrtc offer to server
  private async sendOffer(socket: WebSocket, pc: RTCPeerConnection) {
    try {
      let offer = await pc.createOffer();

      await pc.setLocalDescription(offer);

      let offerMsg = {
        kind: "negotiationRequest",
        offer: pc.localDescription.sdp,
      };
      let offer_string = JSON.stringify(offerMsg);
      socket.send(offer_string);
    } catch (error) {
      console.log("Error sending offer! " + error);
      this.enqueueEvent({ type: "NEGOTIATION_FAILED" });
    }
  }

  private constructWebSocketUrl(): string {
    const loc = window.location;
    const basePath = document.querySelector("base")?.getAttribute("href") || "";
    const protocol = loc.protocol === "https:" ? "wss:" : "ws:";
    return `${protocol}//${loc.host}${basePath}v1/webrtc/connect`;
  }

  /** Handle messages from server over websocket */
  private handleSocketMessage(event: MessageEvent): void {
    const message: ServerMessage = JSON.parse(event.data);
    if (message.kind === "negotiationAnswer") {
      this.enqueueEvent({ type: "NEGOTIATION_ANSWER", answer: message.answer });
    }
  }

  private handleNegotiationAnswer(pc: RTCPeerConnection, answer: string) {
    pc?.setRemoteDescription({
      sdp: answer,
      type: "answer",
    }).catch((err) => {
      console.error("Failed to handle negotiation answer:", err);
      this.disconnect("message error");
    });
  }

  private disconnect(reason: string) {
    console.log("WebRTC disconnected ( " + reason + " )....!");
    this.enqueueEvent({ type: "NEGOTIATION_FAILED" });
  }

  private cleanupTimeout(timeoutId: TimeoutId) {
    clearTimeout(timeoutId);
  }

  private cleanup(signalSocket: WebSocket, pc: RTCPeerConnection) {
    if (signalSocket) {
      signalSocket.onopen = null;
      signalSocket.onclose = null;
      signalSocket.onerror = null;
      signalSocket.onmessage = null;
      signalSocket.close();
    }
    if (pc) {
      pc.onconnectionstatechange = null;
      pc.oniceconnectionstatechange = null;
      pc.onnegotiationneeded = null;
      pc.ontrack = null;
      pc.close();
    }
    this.transceivers.clear();
  }
}
