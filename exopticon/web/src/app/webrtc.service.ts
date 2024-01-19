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

interface Subscription {
    id: number;
    trackId: string;
}

interface id {
    number: Subscription;
    trackId: string;
}

enum Status {
    Paused,
    Connecting,
    Connected,
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
    private signalSocketStatus = Status.Paused;

    // status
    private webrtcStatus: Status = Status.Paused;

    // status timer
    private minTimeout = 500;
    private maxTimeout = 5000;
    private timeoutId: ReturnType<typeof setTimeout>;

    constructor(private http: HttpClient) { }

    updateState() {
        console.log("update state!!! " + this.webrtcStatus + " " + this.enabled);
        clearTimeout(this.timeoutId);

        let s = "closed";
        if (this.peerConnection) {
            s = this.peerConnection.connectionState;
        }
        console.log("WEBRTC STATUS: " + s);
        if (s === "new") {
        } else if (s === "connecting") {
            this.syncTracks();
            //            this.updateSubscriptions();
        } else if (s === "connected") {
            this.syncTracks();
            //            this.updateSubscriptions();
        } else if (s === "disconnected") {
            this.webrtcStatus = Status.Paused;
        } else if (s === "failed") {
            this.webrtcStatus = Status.Paused;
        } else if (s === "closed") {
            //this.webrtcStatus = Status.Paused;
        }

        1    // if enabled but disconnected...
        if (this.enabled && this.webrtcStatus === Status.Paused) {
            console.log("Enabled but paused... connect!");
            this.connect();
        } else if (!this.enabled && this.webrtcStatus !== Status.Paused) {
            this.disconnect();
        } else if (this.enabled && this.webrtcStatus == Status.Connected) {

        }

        if (this.enabled === true) {

            this.timeoutId = setTimeout(this.updateState.bind(this), this.maxTimeout);
        }
    }

    syncTracks() {
        if (this.webrtcStatus === Status.Connected || this.webrtcStatus === Status.Connecting) {
            // ensure every cameraId has a corresponding transceiver
            for (let [cameraId, cameraActive] of this.activeCameras) {
                let direction: RTCRtpTransceiverDirection = cameraActive ? "recvonly" : "inactive";
                let transceiver = this.transceivers.get(cameraId);
                if (transceiver === undefined) {
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

    private updateMappings() {
        if (
            this.signalSocket === undefined ||
            this.signalSocket.readyState !== this.signalSocket.OPEN
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
        return;
        console.log("UPDATTING SUBSCRIPTIONS!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
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

        this.dataChannel = this.peerConnection.createDataChannel("foo");
        this.dataChannel.onclose = () => console.log("sendChannel has closed");
        this.dataChannel.onopen = () => console.log("sendChannel has opened");
        this.dataChannel.onmessage = (e) => { };

        this.peerConnection.onconnectionstatechange = (ev) => {
            switch (this.peerConnection.connectionState) {
                case "new":
                case "connecting":
                    console.log("Connecting...");
                    this.webrtcStatus = Status.Connecting;
                    break;
                case "connected":
                    console.log("Online **");
                    this.webrtcStatus = Status.Connected;
                    break;
                case "disconnected":
                    console.log("Disconnecting...");
                    this.webrtcStatus = Status.Paused;
                    break;
                case "closed":
                    console.log("Connection Offline");
                    this.webrtcStatus = Status.Paused;
                    break;
                case "failed":
                    console.log("Connection Error");
                    this.webrtcStatus = Status.Paused;
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
                    console.log("EMITTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT");
                    this.emitters.get(cameraId).next(stream);
                }
            }
        };
    }

    private async sendOffer() {
        try {
            let offer = await this.peerConnection.createOffer();

            //offer.sdp = offer.sdp.replace("VP8", "H264");

            await this.peerConnection.setLocalDescription(offer);

            let offerMsg = {
                kind: "negotiationRequest",
                offer: this.peerConnection.localDescription.sdp,
            };
            let offer_string = JSON.stringify(offerMsg);
            console.log("Sending new offer over data channel!");
            this.signalSocket.send(offer_string);

        } catch (error) {
            console.log("Error sending offer! " + error);
        }
    }

    private connect() {
        this.webrtcStatus = Status.Connecting;

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

        this.signalSocketStatus = Status.Connecting;

        this.signalSocket.onopen = (event) => {
            this.webrtcConnect();
        };

        this.signalSocket.onclose = (event) => {
            this.disconnect();
        };

        this.signalSocket.onerror = (event) => {
            this.signalSocketStatus = Status.Paused;
        };

        this.signalSocket.onmessage = async (event) => {
            let message: ServerMessage = JSON.parse(event.data);

            try {
                switch (message.kind) {
                    case "negotiationAnswer":
                        await this.peerConnection.setRemoteDescription({
                            sdp: message.answer,
                            type: "answer",
                        });
                        break;
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
    }
}
