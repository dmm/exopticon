import { Injectable } from '@angular/core';
import { Observable, Subject, Subscription } from 'rxjs';
import {mergeMap} from 'rxjs/operators';

import { VideoUnitService } from './video-unit.service';
import { PlaybackSubject, VideoService, SubscriptionSubject } from './video.service';
import { WsMessage, FrameMessage, CameraResolution } from './frame-message';

@Injectable({
  providedIn: 'root'
})
export class VideoClipService {

  constructor(private videoUnitService: VideoUnitService,
              private videoService: VideoService) {}


  playContext(videoUnitId: number, frameOffset: number): Observable<WsMessage> {
    let initialSubject: PlaybackSubject = {
      kind: 'playback',
      id: Math.floor(Math.random() * Math.floor(10000)),
      videoUnitId: videoUnitId,
      offset: frameOffset
    };

    this.videoService.connect();
    return this.videoService.getObservable(initialSubject);


  }
}
