import { Injectable } from '@angular/core';
import { Observable, Subject, Observer } from 'rxjs';
import { webSocket, WebSocketSubject } from 'rxjs/webSocket';

import { FrameMessage } from './frame-message';

@Injectable({
  providedIn: 'root'
})
export class VideoService {
  private subject: WebSocketSubject<FrameMessage>;
  constructor() { }

  public connect(url?: string): WebSocketSubject<FrameMessage> {
    if (!url) {
      let loc = window.location;
      if (loc.protocol === "https:") {
        url = "wss:";
      } else {
        url = "ws:";
      }
      url += "//" + loc.host;
      url += loc.pathname + "v1/ws_json";
      url = 'ws://localhost:3000/v1/ws_json';
    }
    if (!this.subject) {
      this.subject = webSocket(url);
    }

    return this.subject;
  }

  public getObservable(cameraId: number, resolution: string): Observable<FrameMessage> {
    return this.subject.multiplex(
      () => {
        return {
          command: 'subscribe',
          resolution: { type: resolution },
          cameraIds: [cameraId],
        }
      },
      () => {
        return {
          command: 'unsubscribe',
          resolution: { type: resolution },
          cameraIds: [cameraId],
        };
      },
      (m) => m.cameraId === cameraId && m.resolution.type === resolution
    );
  }
}
