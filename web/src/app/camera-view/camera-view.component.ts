import { Component, ChangeDetectorRef, ElementRef, OnInit, Input, SimpleChanges } from '@angular/core';
import { OnPageVisible, OnPageHidden } from 'angular-page-visibility';
import { Observable, Subscription } from 'rxjs';

import { Camera } from '../camera';
import { VideoService } from '../video.service';
import { FrameMessage } from '../frame-message';

@Component({
  selector: 'app-camera-view',
  templateUrl: './camera-view.component.html',
  styleUrls: ['./camera-view.component.css']
})
export class CameraViewComponent implements OnInit {
  @Input() camera: Camera;
  @Input() selected: boolean;
  @Input() videoService: VideoService;
  @Input() active: boolean;

  private frameService?: Observable<FrameMessage>;
  private subscription?: Subscription;
  private img: HTMLImageElement;
  public status: string;

  constructor(private elementRef: ElementRef, private cdr: ChangeDetectorRef) { }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');
    this.status = 'loading';
  }

  ngOnInit() {
    this.status = 'paused';
    this.activate();
  }

  ngOnDestroy() {
    this.deactivate();
  }

  ngOnChanges(changeRecord: SimpleChanges) {
    if (changeRecord.active !== undefined) {
      console.log(changeRecord.active);
      if (this.active) {
        this.activate();
      } else {
        this.deactivate();
      }
    }
  }

  activate() {
    //    this.deactivate();
    this.status = 'loading';
    this.frameService = this.videoService.getObservable(this.camera.id, 'SD');
    this.subscription = this.frameService.subscribe(
      (message) => {
        if (this.status !== 'active') {
          this.status = 'active';
        }
        if (this.img.complete && this.active) {
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
    this.status = 'paused';
    this.cdr.detectChanges();
    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = null;
      this.frameService = null;
    }
  }
}


