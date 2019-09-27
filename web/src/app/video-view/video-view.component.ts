import { SimpleChanges, Component, ChangeDetectorRef, ElementRef, EventEmitter, OnInit, Input, Output, NgZone } from '@angular/core';
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
  @Input() frameService?: Observable<FrameMessage>;

  @Output() status = new EventEmitter<string>();

  private subscription?: Subscription;
  private img: HTMLImageElement;
  private isActive: boolean;

  constructor(private elementRef: ElementRef,
    private cdr: ChangeDetectorRef,
    private ngZone: NgZone) { }

  ngOnInit() {
  }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');
  }

  ngOnChanges(changes: SimpleChanges) {
    if (changes.hasOwnProperty('frameService')) {
      if (changes['frameService'].currentValue) {
        this.activate();
      } else {
        this.deactivate();
      }
    }
  }

  ngOnDestroy() {
    this.deactivate();
  }

  activate() {
    this.isActive = false;
    this.status.emit('loading');

    let oldSubscription = this.subscription;

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

    if (oldSubscription) {
      // still potentially bad if a frame from the old subscription
      // hits first.
      oldSubscription.unsubscribe();
    }

  }

  deactivate() {
    this.status.emit('paused');
    this.isActive = false;

    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = undefined;
    }
  }
}
