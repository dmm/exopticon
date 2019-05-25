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

  @Output() status = new EventEmitter<string>();

  private frameService?: Observable<FrameMessage>;
  private subscription?: Subscription;
  private img: HTMLImageElement;
  private visible: boolean;
  private isActive: boolean;

  constructor(private elementRef: ElementRef, private ngZone: NgZone) { }

  ngOnInit() { }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');

  }

  @OnPageVisible()
  onPageVisible() {
    this.ngZone.run(() => {
      if (this.visible) {
        this.activate();
      }
    });
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
        if (this.img.complete && this.visible) {
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

  onInViewportChange(inViewport: boolean) {
    this.visible = inViewport;
    if (this.visible && this.subscription === undefined) {
      this.activate();
    }
    if (!inViewport) {
      this.deactivate();
    }
  }
}
