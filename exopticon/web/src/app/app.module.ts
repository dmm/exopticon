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

import { APP_BASE_HREF } from "@angular/common";
import {
  HTTP_INTERCEPTORS,
  provideHttpClient,
  withInterceptorsFromDi,
} from "@angular/common/http";
import { NgModule } from "@angular/core";
import { FormsModule, ReactiveFormsModule } from "@angular/forms";
import { BrowserModule } from "@angular/platform-browser";
import { ServiceWorkerModule } from "@angular/service-worker";
import { IntersectionObserverModule } from "@ng-web-apis/intersection-observer";
import { environment } from "../environments/environment";
import { AlertViewComponent } from "./alert-view/alert-view.component";
import { AnalysisPanelComponent } from "./analysis-panel/analysis-panel.component";
import { AppRoutingModule } from "./app-routing.module";
import { AppComponent } from "./app.component";
import { AuthInterceptor } from "./auth.interceptor";
import { CameraDetailComponent } from "./camera-detail/camera-detail.component";
import { CameraGroupDetailComponent } from "./camera-group-detail/camera-group-detail.component";
import { CameraGroupListComponent } from "./camera-group-list/camera-group-list.component";
import { CameraListComponent } from "./camera-list/camera-list.component";
import { CameraOverlayComponent } from "./camera-overlay/camera-overlay.component";
import { CameraPanelComponent } from "./camera-panel/camera-panel.component";
import { CameraStatusOverlayComponent } from "./camera-status-overlay/camera-status-overlay.component";
import { CameraViewComponent } from "./camera-view/camera-view.component";
import { CameraService } from "./camera.service";
import { EventListComponent } from "./event-list/event-list.component";
import { LoginComponent } from "./login/login.component";
import { PlaybackViewComponent } from "./playback-view/playback-view.component";
import { TokenListComponent } from "./token-list/token-list.component";
import { TokenService } from "./token.service";
import { VideoViewComponent } from "./video-view/video-view.component";

@NgModule({
  declarations: [
    AppComponent,
    CameraPanelComponent,
    CameraViewComponent,
    CameraOverlayComponent,
    CameraStatusOverlayComponent,
    AnalysisPanelComponent,
    VideoViewComponent,
    PlaybackViewComponent,
    AlertViewComponent,
    LoginComponent,
    CameraListComponent,
    CameraDetailComponent,
    EventListComponent,
    TokenListComponent,
    CameraGroupListComponent,
    CameraGroupDetailComponent,
  ],
  bootstrap: [AppComponent],
  imports: [
    BrowserModule,
    FormsModule,
    AppRoutingModule,
    IntersectionObserverModule,
    ReactiveFormsModule,
    ServiceWorkerModule.register("ngsw-worker.js", {
      enabled: environment.production,
    }),
  ],
  providers: [
    { provide: HTTP_INTERCEPTORS, useClass: AuthInterceptor, multi: true },
    { provide: APP_BASE_HREF, useValue: "/" },
    CameraService,
    TokenService,
    provideHttpClient(withInterceptorsFromDi()),
  ],
})
export class AppModule {}
