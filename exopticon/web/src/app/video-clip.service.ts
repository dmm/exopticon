import { Injectable } from "@angular/core";
import { Observable } from "rxjs";
import { WsMessage } from "./frame-message";
import { VideoUnitService } from "./video-unit.service";
import { PlaybackSubject, VideoService } from "./video.service";

@Injectable({
  providedIn: "root",
})
export class VideoClipService {
  constructor(
    private videoUnitService: VideoUnitService,
    private videoService: VideoService
  ) {}

  playContext(videoUnitId: number, frameOffset: number): Observable<WsMessage> {
    let initialSubject: PlaybackSubject = {
      kind: "playback",
      id: Math.floor(Math.random() * Math.floor(10000)),
      videoUnitId: videoUnitId,
      offset: frameOffset,
    };

    this.videoService.connect();
    return this.videoService.getObservable(initialSubject);
  }
}
