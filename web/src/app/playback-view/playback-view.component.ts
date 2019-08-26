import { Component, OnInit } from '@angular/core';

import { SubscriptionSubject, VideoService } from '../video.service';

@Component({
  selector: 'app-playback-view',
  templateUrl: './playback-view.component.html',
  styleUrls: ['./playback-view.component.css']
})
export class PlaybackViewComponent implements OnInit {
  public enabled: boolean;
  public videoSubject: SubscriptionSubject;

  constructor(public videoService: VideoService) { }

  ngOnInit() {
    this.videoService.connect();
    this.enabled = true;
    this.videoSubject = {
      kind: 'playback',
      id: 1
    }
  }

}
