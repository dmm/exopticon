/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

import { Component, Input, OnInit } from "@angular/core";
import { ActivatedRoute } from "@angular/router";
import { Observable } from "rxjs";
import { WsMessage } from "../frame-message";
import { SubscriptionSubject, VideoService } from "../video.service";

@Component({
  selector: "app-analysis-panel",
  templateUrl: "./analysis-panel.component.html",
  styleUrls: ["./analysis-panel.component.css"],
})
export class AnalysisPanelComponent implements OnInit {
  @Input() analysisEngineId: number;

  public videoSubject: SubscriptionSubject;
  public frameService?: Observable<WsMessage>;

  constructor(
    public route: ActivatedRoute,
    public videoService: VideoService
  ) {}

  ngOnInit() {
    this.videoService.connect();
    let id = this.route.snapshot.paramMap.get("id");
    this.videoSubject = {
      kind: "analysisEngine",
      analysisEngineId: parseInt(id, 10),
    };
    this.frameService = this.videoService.getObservable(this.videoSubject);
  }
}
