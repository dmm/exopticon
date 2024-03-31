/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
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

import { EventEmitter, Injectable } from "@angular/core";
import { Observable, Subject, Subscription } from "rxjs";
import { webSocket, WebSocketSubject } from "rxjs/webSocket";
import { CameraResolution, FrameMessage, WsMessage } from "./frame-message";

export interface CameraSubject {
  kind: "camera";
  cameraId: number;
  resolution: CameraResolution;
}

export interface AnalysisSubject {
  kind: "analysisEngine";
  analysisEngineId: number;
}

export interface PlaybackSubject {
  kind: "playback";
  id: number;
  videoUnitId: string;
  offset: number;
}

export type SubscriptionSubject =
  | AnalysisSubject
  | CameraSubject
  | PlaybackSubject;

@Injectable({
  providedIn: "root",
})
export class VideoService {
  private subject: WebSocketSubject<Object>;
  private subscriberCount = 0;
  private subscription?: Subscription;
  private errorObservable: EventEmitter<string> = new EventEmitter<string>();

  constructor() {
    this.subscription = null;
  }

  public connect(url?: string): WebSocketSubject<Object> {
    if (!url) {
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
    }
    if (!this.subject) {
      this.subject = webSocket(url);
    }
    return this.subject;
  }

  public getErrorObservable(): EventEmitter<string> {
    return this.errorObservable;
  }

  private setupAcker() {
    if (this.subscription === null) {
      this.subscription = this.subject.subscribe(
        (frame: FrameMessage) => {
          if (frame.kind === "frame") {
            this.subject.next("Ack");
          }
        },
        () => {
          this.errorObservable.emit("error!");
          this.subscription = null;
          this.setupAcker();
        },
        () => {},
      );
    }
  }

  private cleanupAcker() {
    if (this.subscriberCount == 0 && this.subscription) {
      setTimeout(() => {
        if (this.subscriberCount == 0 && this.subscription) {
          // if after a moment, there are still zero subscribers, close the socket.
          this.subscription.unsubscribe();
          this.subscription = null;
        }
      }, 100);
    }
  }

  public getObservable(subject: SubscriptionSubject): Observable<WsMessage> {
    return null;
    let frameSub: WebSocketSubject<WsMessage> = this
      .subject as unknown as WebSocketSubject<WsMessage>;

    return frameSub.multiplex(
      () => {
        this.setupAcker();
        this.subscriberCount++;
        switch (subject.kind) {
          case "camera":
            return {
              Subscribe: {
                Camera: [subject.cameraId, subject.resolution],
              },
            };
          case "analysisEngine":
            return {
              Subscribe: {
                AnalysisEngine: subject.analysisEngineId,
              },
            };
          case "playback":
            return {
              StartPlayback: {
                id: subject.id,
                video_unit_id: subject.videoUnitId,
                offset: subject.offset,
              },
            };
        }
      },
      () => {
        this.subscriberCount--;
        this.cleanupAcker();
        switch (subject.kind) {
          case "camera":
            return {
              Unsubscribe: {
                Camera: [subject.cameraId, subject.resolution],
              },
            };
          case "analysisEngine":
            return {
              Unsubscribe: {
                AnalysisEngine: subject.analysisEngineId,
              },
            };
          case "playback":
            return {
              StopPlayback: {
                id: subject.id,
              },
            };
        }
      },
      (m: WsMessage): boolean => {
        if (m.kind === "frame") {
          switch (m.source.kind) {
            case "camera":
              return (
                subject.kind === "camera" &&
                subject.cameraId === m.source.cameraId &&
                subject.resolution === m.resolution
              );
            case "analysisEngine":
              return (
                subject.kind === "analysisEngine" &&
                subject.analysisEngineId === m.source.analysisEngineId
              );

            case "playback":
              return subject.kind === "playback" && subject.id === m.source.id;
          }
        } else if (m.kind === "playbackEnd") {
          return subject.kind === "playback" && subject.id == m.id;
        } else {
          // invalid kind
        }
      },
    );
  }

  public getWriteSubject(): Subject<Object> {
    return this.subject;
  }
}
