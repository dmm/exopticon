import { Component, ChangeDetectorRef, ElementRef, OnInit, Input } from '@angular/core';
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

  private frameService?: Observable<FrameMessage>;
  private subscription?: Subscription;
  private img: HTMLImageElement;
  private inViewport: boolean;
  private status: string;

  constructor(private elementRef: ElementRef, private cdr: ChangeDetectorRef) { }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');
    this.status = 'loading';
  }

  ngOnInit() {
    this.status = 'paused';
    this.inViewport = true;

    this.activate();
  }

  ngOnDestroy() {
    this.deactivate();
  }

  @OnPageVisible()
  activate() {
    if (this.inViewport) {
      this.deactivate();
      this.status = 'loading';
      this.frameService = this.videoService.getObservable(this.camera.id, 'SD');
      this.subscription = this.frameService.subscribe(
        (message) => {
          if (this.status !== 'active') {
            this.status = 'active';
            this.cdr.detectChanges(); // WTF?
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
  }

  @OnPageHidden()
  deactivate() {
    this.status = 'paused';
    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = null;
      this.frameService = null;
    }
  }

  onInViewportChange(inViewport: boolean) {
    this.inViewport = inViewport;
    if (inViewport) {
      this.activate();
    } else {
      this.deactivate();
    }
  }

}


