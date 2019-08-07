import { Component, ChangeDetectorRef, ElementRef, EventEmitter, OnInit, Input, Output, NgZone } from '@angular/core';
import { OnPageVisible, OnPageHidden } from 'angular-page-visibility';
import { Observable, Subscription } from 'rxjs';

import { Camera } from '../camera';
import { CameraResolution, FrameMessage } from '../frame-message';
import { SubscriptionSubject, VideoService } from '../video.service';

@Component({
  selector: 'app-video-view',
  templateUrl: './video-view.component.html',
  styleUrls: ['./video-view.component.css']
})
export class VideoViewComponent implements OnInit {
  @Input() videoService: VideoService;
  @Input() videoSubject: SubscriptionSubject;
  @Input() set enabled(value: boolean) {
    console.log(`Video view enable: ${value}`);
    if (value && this.ready) {
      setTimeout(() => {
        this.activate();
      }, 0)
    } else {
      if (this.ready) {
        setTimeout(() => {
          this.deactivate();
        }, 0);
      }
    }
  }

  @Output() status = new EventEmitter<string>();

  private frameService?: Observable<FrameMessage>;
  private subscription?: Subscription;
  private img: HTMLImageElement;
  private ready: boolean;
  private isActive: boolean;

  constructor(private elementRef: ElementRef,
    private cdr: ChangeDetectorRef,
    private ngZone: NgZone) { }

  ngOnInit() {
    this.ready = true;
    this.activate(); // !!!! remove this

  }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');

  }

  ngOnDestroy() {
    this.deactivate();
  }

  activate() {
    this.isActive = false;
    this.status.emit('loading');

    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = undefined;
      this.frameService = undefined;
    }

    this.frameService = this.videoService.getObservable(this.videoSubject);

    this.subscription = this.frameService.subscribe(
      (message) => {
        if (!this.isActive) {
          this.isActive = true;
          this.status.emit('active');
        }
        if (this.img.complete) {
          this.img.onerror = () => { console.log("error!"); };
          this.img.src = `data:image/jpeg;base64, ${message.jpeg}`;
        }
      },
      (error) => {
        console.log(`Caught websocket error! ${error}`);
      },
    );
  }

  deactivate() {
    this.status.emit('paused');
    this.isActive = false;

    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = undefined;
      this.frameService = undefined;
    }
  }
}
