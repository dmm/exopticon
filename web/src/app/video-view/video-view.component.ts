import { SimpleChanges, ViewChild, Component, ChangeDetectorRef, ElementRef, EventEmitter, OnInit, Input, Output, NgZone } from '@angular/core';
import { OnPageVisible, OnPageHidden } from 'angular-page-visibility';
import { Observable, Subscription } from 'rxjs';

import { Camera } from '../camera';
import { CameraResolution, FrameMessage } from '../frame-message';
import { SubscriptionSubject, VideoService } from '../video.service';
import { Observation } from '../observation';

@Component({
  selector: 'app-video-view',
  templateUrl: './video-view.component.html',
  styleUrls: ['./video-view.component.css']
})
export class VideoViewComponent implements OnInit {
  @Input() frameService?: Observable<FrameMessage>;
  @Output() status = new EventEmitter<string>();

  @ViewChild('obsCanvas', { static: true })
  canvas: ElementRef<HTMLCanvasElement>;

  private subscription?: Subscription;
  private img: HTMLImageElement;
  private isActive: boolean;
  private ctx: CanvasRenderingContext2D;

  constructor(private elementRef: ElementRef,
    private cdr: ChangeDetectorRef,
    private ngZone: NgZone) { }

  ngOnInit() {
    this.ctx = this.canvas.nativeElement.getContext('2d');
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
          this.drawObservations(message.observations);
        }
      },
      (error) => {
        console.log(`Caught websocket error! ${error}`);
      },
    );

    console.log(`old subscription: ${oldSubscription}`);
    if (oldSubscription) {
      // still potentially bad if a frame from the old subscription
      // hits first.
      console.log('removing old subscription');
      oldSubscription.unsubscribe();
    }

  }

  drawObservations(observations: Observation[]) {
    observations.forEach((o) => {
      console.log("Rendering: " + o);
      this.ctx.fillStyle = 'green';
      //      this.ctx.clearRect(0, 0, this.canvas.nativeElement.width, this.canvas.nativeElement.height);
      let width = o.lrX - o.ulX;
      let height = o.lrY - o.ulY;
      this.ctx.strokeRect(o.ulX, o.ulY, width, height);
    });
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
