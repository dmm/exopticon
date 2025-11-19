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

type TimeoutId = ReturnType<typeof setTimeout>;

type ClientStatus =
  | Paused
  | SignalChannelConnecting
  | WebrtcConnecting
  | WebrtcConnected;

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
  | { type: "ICE_CONNECTED" }
  | { type: "ICE_DISCONNECTED" }
  | { type: "ICE_FAILED" }
  | { type: "NEGOTIATION_NEEDED" }
  | { type: "NEGOTIATION_ANSWER"; answer: string }
  | {
      type: "TRACK_RECEIVED";
      transceiver: RTCRtpTransceiver;
      stream: MediaStream;
    }
  | { type: "TIMEOUT"; state: State }
  | { type: "UPDATE_CAMERAS"; cameras: CameraId[] };

// States
type State =
  | { kind: "disabled" }
  | {
      kind: "connecting_signal";
      since: Instant;
      timeoutId: TimeoutId;
    }
  | { kind: "connecting_webrtc"; since: Instant; timeoutId: TimeoutId }
  | { kind: "connected"; since: Instant }
  | { kind: "reconnecting"; attempt: number; timeoutId: TimeoutId };

@Injectable({
  providedIn: "root",
})
export class WebrtcService {
  private state: State = { kind: "disabled" };
  private eventQueue: WebRtcEvent[] = [];
  private processing = false;

  private peerConnection: RTCPeerConnection;
  private signalSocket?: WebSocket;

  private dataChannel?: RTCDataChannel;
  private transceivers: Map<CameraId, RTCRtpTransceiver> = new Map();
  private emitters: Map<CameraId, ReplaySubject<MediaStream>> = new Map();
  private activeCameras: Map<CameraId, boolean> = new Map();

  statusSubject: BehaviorSubject<State> = new BehaviorSubject(this.state);

  // status timer
  private readonly baseDelay = 100;
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

  private enqueueEvent(event: WebRtcEvent): void {
    this.eventQueue.push(event);
    this.processEvents();
  }

  private async processEvents(): Promise<void> {
    if (this.processing) return;
    this.processing = true;

    while (this.eventQueue.length > 0) {
      const event = this.eventQueue.shift()!;
      await this.handleEvent(event);
    }

    this.processing = false;
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

      // Handle state entry actions
      await this.onStateEnter(this.state);
    }
  }

  // State handlers
  private handleDisabledState(event: WebRtcEvent): State {
    if (this.state.kind !== "disabled") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "ENABLE":
        return {
          kind: "connecting_signal",
          since: Instant.now(),
          timeoutId: null,
        };
      case "DISABLE":
        return this.state;
      default:
        console.warn(`Ignoring event ${event.type}, in disabled state.`);
        return this.state;
    }
  }

  private async handleConnectingSignalState(
    event: WebRtcEvent,
  ): Promise<State> {
    if (this.state.kind !== "connecting_signal") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "DISABLE":
        this.cleanupTimeout(this.state.timeoutId);
        await this.cleanup();
        return { kind: "disabled" };

      case "UPDATE_CAMERAS":
        console.log(`ACTIVE CAMERAS: ${event.cameras}`);
        this.syncTracks(event.cameras);
        return this.state;

      case "WEBSOCKET_OPEN":
        this.cleanupTimeout(this.state.timeoutId);
        return {
          kind: "connecting_webrtc",
          since: Instant.now(),
          timeoutId: null,
        };

      case "WEBSOCKET_CLOSE":
      case "WEBSOCKET_ERROR":
        this.cleanupTimeout(this.state.timeoutId);
        return { kind: "reconnecting", attempt: 1, timeoutId: null };

      case "TIMEOUT":
        console.warn("Signal connection timeout");
        await this.cleanup();
        return { kind: "reconnecting", attempt: 1, timeoutId: null };

      default:
        return this.state;
    }
  }

  private async handleConnectingWebrtcState(
    event: WebRtcEvent,
  ): Promise<State> {
    if (this.state.kind !== "connecting_webrtc") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "DISABLE":
        this.cleanupTimeout(this.state.timeoutId);
        await this.cleanup();
        return { kind: "disabled" };

      case "UPDATE_CAMERAS":
        console.log(`ACTIVE CAMERAS: ${event.cameras}`);
        this.syncTracks(event.cameras);
        this.updateStreamMappings();
        return this.state;

      case "WEBRTC_CONNECTED":
        this.cleanupTimeout(this.state.timeoutId);
        return { kind: "connected", since: Instant.now() };

      case "WEBRTC_FAILED":
      case "WEBSOCKET_CLOSE":
        this.cleanupTimeout(this.state.timeoutId);
        await this.cleanup();
        return { kind: "reconnecting", attempt: 1, timeoutId: null };

      case "TIMEOUT":
        console.warn("WebRTC connection timeout");
        await this.cleanup();
        return { kind: "reconnecting", attempt: 1, timeoutId: null };

      case "NEGOTIATION_ANSWER":
        await this.handleNegotiationAnswer(event.answer);
        return this.state;

      default:
        return this.state;
    }
  }

  private async handleConnectedState(event: WebRtcEvent): Promise<State> {
    if (this.state.kind !== "connected") {
      console.error(`invalid state handler called: ${this.state.kind}`);
      return;
    }

    switch (event.type) {
      case "DISABLE":
        await this.cleanup();
        return { kind: "disabled" };

      case "UPDATE_CAMERAS":
        console.log(`ACTIVE CAMERAS: ${event.cameras}`);
        this.syncTracks(event.cameras);
        this.updateStreamMappings();
        return this.state;

      case "WEBRTC_CONNECTED":
        // We got a WEBRTC_CONNECTED when we're already in the
        // connected state. This happens when we renegotiate another
        // stream, so update the stream mappings.
        const map1 = new Map(
          [...this.activeCameras].filter(([_k, v]) => v === true),
        );
        this.syncTracks(Array.from(map1.keys()));

        this.updateStreamMappings();
        return this.state;

      case "WEBRTC_DISCONNECTED":
      case "WEBRTC_FAILED":
      case "WEBSOCKET_CLOSE":
        await this.cleanup();
        return { kind: "reconnecting", attempt: 1, timeoutId: null };

      case "NEGOTIATION_ANSWER":
        await this.handleNegotiationAnswer(event.answer);
        return this.state;

      default:
        return this.state;
    }
  }

  private async handleReconnectingState(event: WebRtcEvent): Promise<State> {
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
        if (this.state.attempt < 5) {
          return {
            kind: "connecting_signal",
            since: Instant.now(),
            timeoutId: null,
          };
        } else {
          console.error("Max reconnection attempts reached");
          return { kind: "disabled" };
        }

      default:
        return this.state;
    }
  }

  // State entry handler
  private async onStateEnter(state: State): Promise<void> {
    switch (state.kind) {
      case "disabled":
        // Ensure cleanup
        await this.cleanup();
        break;

      case "connecting_signal":
        this.setupSignalSocket();
        // Set timeout for this state
        const timeoutId = setTimeout(() => {
          this.enqueueEvent({ type: "TIMEOUT", state });
        }, this.maxTimeout);
        this.state = { ...state, timeoutId };
        break;

      case "connecting_webrtc":
        this.webrtcConnect();
        const webrtcTimeoutId = setTimeout(() => {
          this.enqueueEvent({ type: "TIMEOUT", state });
        }, this.maxTimeout);
        this.state = { ...state, timeoutId: webrtcTimeoutId };
        break;

      case "connected":
        const map1 = new Map(
          [...this.activeCameras].filter(([_k, v]) => v === true),
        );
        this.syncTracks(Array.from(map1.keys()));

        this.updateStreamMappings();
        break;

      case "reconnecting":
        const delay = Math.min(1000 * Math.pow(2, state.attempt - 1), 30000);
        const reconnectTimeoutId = setTimeout(() => {
          this.enqueueEvent({ type: "TIMEOUT", state });
        }, delay);
        this.state = { ...state, timeoutId: reconnectTimeoutId };
        break;
    }
  }

  private setupSignalSocket(): void {
    const url = this.constructWebSocketUrl();
    this.signalSocket = new WebSocket(url);

    this.signalSocket.onopen = () =>
      this.enqueueEvent({ type: "WEBSOCKET_OPEN" });
    this.signalSocket.onclose = () =>
      this.enqueueEvent({ type: "WEBSOCKET_CLOSE", reason: "closed" });
    this.signalSocket.onerror = () =>
      this.enqueueEvent({ type: "WEBSOCKET_ERROR" });
    this.signalSocket.onmessage = (event) => this.handleSocketMessage(event);
  }

  private hasExceededTimeout(since: Instant): boolean {
    const duration = Duration.between(since, Instant.now()).toMillis();
    return duration > this.maxTimeout;
  }

  /** Updates mappings between transceivers and cameras. */
  private updateStreamMappings(): void {
    const mappings: Record<string, CameraId> = {};
    for (const [cameraId, transceiver] of this.transceivers) {
      if (transceiver.mid && this.activeCameras.get(cameraId)) {
        mappings[transceiver.mid] = cameraId;
      }
    }

    this.signalSocket.send(JSON.stringify({ kind: "streamMapping", mappings }));
  }

  /** Synchronizes transceivers with active cameras. */
  private syncTracks(activeCameras: CameraId[]): void {
    for (let [id, _val] of this.activeCameras) {
      this.activeCameras.set(id, false);
    }

    for (const cameraId of activeCameras) {
      this.activeCameras.set(cameraId, true);
      if (!this.transceivers.has(cameraId)) {
        const transceiver = this.peerConnection?.addTransceiver("video", {
          direction: "recvonly",
        });
        this.transceivers.set(cameraId, transceiver!);
      }
    }
  }

  /** Inititates WebRTC connection */
  private webrtcConnect() {
    this.peerConnection = new RTCPeerConnection();

    this.dataChannel = this.peerConnection.createDataChannel("foo");
    this.dataChannel.onclose = () => {};
    this.dataChannel.onopen = () => {};

    this.dataChannel.onmessage = (_) => {};

    this.peerConnection.onconnectionstatechange = () => {
      console.log(
        `CONNECTION STATE CHANGE: ${this.peerConnection.connectionState}`,
      );
      switch (this.peerConnection?.connectionState) {
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

    this.peerConnection.oniceconnectionstatechange = (e) => {
      console.log(
        `ICE CONNECTION STATE CHANGE: ${this.peerConnection.iceConnectionState}`,
      );

      let state = this.peerConnection.iceConnectionState;
      if (state === "connected") {
      } else if (state === "disconnected" || state === "failed") {
      }
      //      this.updateState();
    };

    this.peerConnection.onnegotiationneeded = async (_e) => {
      console.log(`ICE negotiation requested`);
      this.sendOffer();
    };

    this.peerConnection.onicecandidate = (event) => {
      if (event.candidate !== null) {
      }
    };

    this.peerConnection.ontrack = ({ transceiver, streams: [stream] }) => {
      for (const [cameraId, tran] of this.transceivers) {
        if (tran.mid === transceiver.mid) {
          console.log(`FETCHING EMITTER FOR CAMERA ID: ${cameraId}`);
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

  /** Handle messages from server over websocket */
  private handleSocketMessage(event: MessageEvent): void {
    const message: ServerMessage = JSON.parse(event.data);
    if (message.kind === "negotiationAnswer") {
      this.handleNegotiationAnswer(message.answer);
    }
  }

  private async handleNegotiationAnswer(answer: string): Promise<void> {
    try {
      await this.peerConnection?.setRemoteDescription({
        sdp: answer,
        type: "answer",
      });
    } catch (err) {
      console.error("Failed to handle negotiation answer:", err);
      this.disconnect("message error");
    }
  }

  private async disconnect(reason: string) {
    console.log("WebRTC disconnected ( " + reason + " )....!");
  }

  private cleanupTimeout(timeoutId: TimeoutId) {
    clearTimeout(timeoutId);
  }

  private async cleanup() {
    if (this.signalSocket) {
      this.signalSocket.close();
    }
    this.signalSocket.close();
    if (this.peerConnection) {
      this.peerConnection.close();
      this.peerConnection = undefined;
      this.dataChannel = undefined;
    }
    this.transceivers.clear();
  }
}
