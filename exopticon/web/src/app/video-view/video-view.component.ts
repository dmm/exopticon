import { SimpleChanges, ViewChild, Component, ChangeDetectorRef, ElementRef, EventEmitter, OnInit, Input, Output, NgZone } from '@angular/core';
import { Observable, Subscription } from 'rxjs';

import { Camera } from '../camera';
import { CameraResolution, WsMessage } from '../frame-message';
import { SubscriptionSubject, VideoService } from '../video.service';
import { Observation } from '../observation';

@Component({
  selector: 'app-video-view',
  templateUrl: './video-view.component.html',
  styleUrls: ['./video-view.component.css']
})
export class VideoViewComponent implements OnInit {
  @Input() frameService?: Observable<WsMessage>;
  @Output() status = new EventEmitter<string>();
  @Output() frameOffset = new EventEmitter<number>();

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

  }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');
    this.ctx = this.canvas.nativeElement.getContext('2d');
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

    let oldSubscription = undefined; //this.subscription;
    if (this.subscription) {
      this.subscription.unsubscribe();
    }
    this.subscription = this.frameService.subscribe(
      (message: WsMessage) => {
        if (message.kind === 'playbackEnd') {
          console.log('VideoView: playback End: ' + message.id);
          this.status.emit('eof');
        } else if (message.kind = 'frame') {
          if (!this.isActive) {
            this.isActive = true;
            this.status.emit('active');
          }
          if (this.img.complete) {
            this.img.onerror = () => { console.log("error!"); };
            this.img.src = `data:image/jpeg;base64, ${message.jpeg}`;
            this.frameOffset.emit(message.offset);
            this.drawObservations(message.unscaledWidth, message.unscaledHeight, message.observations);
          }
        }
      },
      (error) => {
        console.log(`Caught websocket error! ${error}`);
      },
      () => {
        // complete
        console.log('VideoView: playback End subscription complete');
        this.status.emit('eof');
      }
    );

    console.log(`old subscription: ${oldSubscription}`);
    if (oldSubscription) {
      // still potentially bad if a frame from the old subscription
      // hits first.
      console.log('removing old subscription');
      oldSubscription.unsubscribe();
    }

  }

  drawObservations(canvasWidth: number, canvasHeight: number, observations: Observation[]) {
    //    this.ctx.clearRect(0, 0, this.canvas.nativeElement.width, this.canvas.nativeElement.height);
    this.canvas.nativeElement.width = canvasWidth;
    this.canvas.nativeElement.height = canvasHeight;
    this.ctx.strokeStyle = '#0F0';
    this.ctx.fillStyle = '#0F0';
    this.ctx.lineWidth = 5.0;
    this.ctx.font = '32pt sans';

    observations.forEach((o) => {
      if (o.tag == 'motion') return;
      let width = o.lrX - o.ulX;
      let height = o.lrY - o.ulY;
      this.ctx.strokeRect(o.ulX, o.ulY, width, height);
      this.ctx.fillText(o.details, o.ulX, o.ulY);
      this.ctx.fillText(o.score.toString(), o.lrX, o.lrY + 40);
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
