import { Component, OnInit } from '@angular/core';
import { VideoService } from '../video.service';

@Component({
  selector: 'app-analysis-panel',
  templateUrl: './analysis-panel.component.html',
  styleUrls: ['./analysis-panel.component.css']
})
export class AnalysisPanelComponent implements OnInit {

  constructor(private videoService: VideoService) { }

  ngOnInit() {
  }

}
