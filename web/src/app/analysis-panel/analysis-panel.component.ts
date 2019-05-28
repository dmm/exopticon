import { Component, OnInit, Input } from '@angular/core';
import { SubscriptionSubject, VideoService } from '../video.service';
import { switchMap } from 'rxjs/operators';
import { Router, ActivatedRoute, ParamMap } from '@angular/router';

@Component({
  selector: 'app-analysis-panel',
  templateUrl: './analysis-panel.component.html',
  styleUrls: ['./analysis-panel.component.css']
})
export class AnalysisPanelComponent implements OnInit {
  @Input() analysisEngineId: number;

  public videoSubject: SubscriptionSubject;

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
  }

}
