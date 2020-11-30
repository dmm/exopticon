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

import { NgModule } from "@angular/core";
import { RouterModule, Routes } from "@angular/router";
import { AlertViewComponent } from "./alert-view/alert-view.component";
import { AnalysisPanelComponent } from "./analysis-panel/analysis-panel.component";
import { CameraPanelComponent } from "./camera-panel/camera-panel.component";
import { PlaybackViewComponent } from "./playback-view/playback-view.component";

const routes: Routes = [
  { path: "cameras", component: CameraPanelComponent },
  { path: "analysis_engine/:id", component: AnalysisPanelComponent },
  { path: "cameras/:id/playback", component: PlaybackViewComponent },
  { path: "alerts/:id", component: AlertViewComponent },
  { path: "", redirectTo: "/cameras", pathMatch: "full" },
  { path: "**", redirectTo: "/cameras", pathMatch: "full" },
];

@NgModule({
  imports: [RouterModule.forRoot(routes, { relativeLinkResolution: 'legacy' })],
  exports: [RouterModule],
})
export class AppRoutingModule {}
