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
import { CameraDetailComponent } from "./camera-detail/camera-detail.component";
import { CameraPanelComponent } from "./camera-panel/camera-panel.component";
import { EventListComponent } from "./event-list/event-list.component";
import { LoginComponent } from "./login/login.component";
import { TokenListComponent } from "./token-list/token-list.component";
import { WebrtcCodecsComponent } from "./webrtc-codecs/webrtc-codecs.component";

import { PlaybackComponent } from "./playback/playback.component";

import { ConfigurationComponent } from "./configuration/configuration.component";

const routes: Routes = [
  {
    path: "configuration",
    redirectTo: "configuration/storage_groups",
    pathMatch: "full",
  },
  {
    path: "configuration/:resource/new",
    component: ConfigurationComponent,
    data: { mode: "new" },
  },
  {
    path: "configuration/:resource/:name/edit",
    component: ConfigurationComponent,
    data: { mode: "edit" },
  },
  {
    path: "configuration/:resource/:name/delete",
    component: ConfigurationComponent,
    data: { mode: "delete" },
  },
  {
    path: "configuration/:resource",
    component: ConfigurationComponent,
    data: { mode: "list" },
  },
  { path: "playback/:camera_name", component: PlaybackComponent },
  { path: "login", component: LoginComponent },
  { path: "camera_panel", component: CameraPanelComponent },
  { path: "cameras", redirectTo: "configuration/cameras", pathMatch: "full" },
  { path: "cameras/:name", component: CameraDetailComponent },
  {
    path: "camera_groups",
    redirectTo: "configuration/camera_groups",
    pathMatch: "full",
  },
  {
    path: "camera_groups/:name",
    redirectTo: "configuration/camera_groups/:name/edit",
  },
  { path: "webrtc_codecs", component: WebrtcCodecsComponent },
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
