import { EventEmitter, Injectable, ElementRef } from '@angular/core';
import { Observable, Subject, Subscription } from 'rxjs';
import { Duration, Instant } from '@js-joda/core'

import { VideoService, SubscriptionSubject } from './video.service';
import { VideoUnitService } from './video-unit.service';
import { WsMessage } from './frame-message';

export class VideoUnitLibrary {
  private availability: [Instant, Instant][];

  constructor(private videoUnitService: VideoUnitService) { }

  getAvailability(): [Instant, Instant][] {
    return [];
  }

  getNext(unit: VideoUnit?) {
    let i =;this.videoUnits.findIndex((u) => {
      return u.id == unit.id;
    });
    if (i > -1 && i < this.videoUnits.length) {
      return this.videoUnits[i+1];
    } else {
      return undefined;
    }
  }
}

export class VideoPlayer {
  private library: VideoUnitLibrary = new VideoUnitLibrary([]);
  private position: Instant;
  private subject?: SubscriptionSubject;
  private subscription?: Subscription;
  private videoObservable: Observable<WsMessage>;
  private positionEmitter: EventEmitter<Instant> = new EventEmitter<Instant>(true);
  private frameEmitter: EventEmitter<FrameMessage> = new EventEmitter<FrameMessage>(true);

  constructor(private frameService: Observable<WsMessage>,
              private videoUnitService: VideoUnitService,
              private videoService: VideoService,
              private canvas: ElementRef<HTMLCanvasElement>) {
    this.position = Instant.MIN;
  }

  start(cameraId: number, time: Instant): EventEmitter<FrameMessage> {
    if (this.subscription) {
      this.pause();
    }
    position = time;
    // look up video unit
    
    this.subscription = this.frameService.subscribe((message: WsMessage) => {
      
    });
  }

  pause() {
    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = undefined;
    }
  }

  resume() {
    this.start(this.position);
  }

  getPlayPosition(): EventEmitter<Instant> {
  }

  
  private processAvailability(observer) {
    this.videoUnitService.getVideoUnits(cameraId, beginTime.atZone(ZoneId.of('Z')), endTime.atZone(ZoneId.of('Z')))
    .subscribe((units) => {
      this.videoUnitLibrary = new VideoUnitLibrary(units);
      

    });
    
  }

  getAvailability(cameraId: number,
                  beginTime: Instant,
                  endTime: Instant): Observable<[Instant, Instant][]>
  {
  }

}
