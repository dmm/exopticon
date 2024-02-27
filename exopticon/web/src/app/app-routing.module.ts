/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2022 David Matthew Mattli <dmm@mattli.us>
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

import { NgModule } from "@angular/core";
import { RouterModule, Routes } from "@angular/router";
import { AlertViewComponent } from "./alert-view/alert-view.component";
import { AnalysisPanelComponent } from "./analysis-panel/analysis-panel.component";
import { CameraDetailComponent } from "./camera-detail/camera-detail.component";
import { CameraGroupDetailComponent } from "./camera-group-detail/camera-group-detail.component";
import { CameraGroupListComponent } from "./camera-group-list/camera-group-list.component";
import { CameraListComponent } from "./camera-list/camera-list.component";
import { CameraPanelComponent } from "./camera-panel/camera-panel.component";
import { EventListComponent } from "./event-list/event-list.component";
import { LoginComponent } from "./login/login.component";
import { PlaybackViewComponent } from "./playback-view/playback-view.component";
import { TokenListComponent } from "./token-list/token-list.component";

const routes: Routes = [
  { path: "login", component: LoginComponent },
  { path: "camera_panel", component: CameraPanelComponent },
  { path: "analysis_engine/:id", component: AnalysisPanelComponent },
  { path: "cameras", component: CameraListComponent },
  { path: "cameras/:id", component: CameraDetailComponent },
  { path: "cameras/:id/playback", component: PlaybackViewComponent },
  { path: "camera_groups", component: CameraGroupListComponent },
  { path: "camera_groups/new", component: CameraGroupDetailComponent },
  { path: "camera_groups/:id", component: CameraGroupDetailComponent },
  { path: "alerts/:id", component: AlertViewComponent },
  { path: "events", component: EventListComponent },
  { path: "tokens", component: TokenListComponent },
  { path: "", redirectTo: "/camera_panel", pathMatch: "full" },
  { path: "**", redirectTo: "/camera_panel", pathMatch: "full" },
];

@NgModule({
  imports: [RouterModule.forRoot(routes)],
  exports: [RouterModule],
})
export class AppRoutingModule {}
