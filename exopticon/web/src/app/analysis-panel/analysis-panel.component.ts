import { Component, OnInit, Input } from '@angular/core';

import { Observable } from 'rxjs';
import { switchMap } from 'rxjs/operators';
import { Router, ActivatedRoute, ParamMap } from '@angular/router';

import { WsMessage } from '../frame-message';
import { SubscriptionSubject, VideoService } from '../video.service';

@Component({
  selector: 'app-analysis-panel',
  templateUrl: './analysis-panel.component.html',
  styleUrls: ['./analysis-panel.component.css']
})
export class AnalysisPanelComponent implements OnInit {
  @Input() analysisEngineId: number;

  public videoSubject: SubscriptionSubject;
  public frameService?: Observable<WsMessage>;

  constructor(public route: ActivatedRoute,
    public videoService: VideoService) { }

  ngOnInit() {
    this.videoService.connect();
    let id = this.route.snapshot.paramMap.get('id');
    this.videoSubject =
      {
        kind: 'analysisEngine',
        analysisEngineId: parseInt(id, 10),
      };
    this.frameService = this.videoService.getObservable(this.videoSubject);
  }

}
