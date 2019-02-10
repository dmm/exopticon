import { Component, ChangeDetectorRef, ElementRef, OnInit, Input, NgZone } from '@angular/core';
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
  private visible: boolean;

  constructor(private elementRef: ElementRef, private ngZone: NgZone) {
    this.status = 'paused';
  }

  ngOnInit() { }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');

  }

  ngOnDestroy() {
    this.deactivate();
  }

  ngDoCheck() {
  }

  @OnPageVisible()
  onPageVisible() {
    if (this.visible && this.active) {
      this.activate();
    }
  }

  @OnPageHidden()
  onPageHidden() {
    this.deactivate();
  }

  activate() {
    this.status = 'loading';
    this.deactivate();
    this.frameService = this.videoService.getObservable(this.camera.id, 'SD');
    this.subscription = this.frameService.subscribe(
      (message) => {
        if (this.status !== 'active') {
          this.ngZone.run(() => this.status = 'active');;

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

    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = undefined;
      this.frameService = undefined;
    }
  }

  onInViewportChange(inViewport: boolean) {
    this.visible = inViewport;
    if (this.visible && this.active && this.subscription === undefined) {
      this.activate();
    }
    if (!inViewport) {
      this.deactivate();
    }
  }
}


