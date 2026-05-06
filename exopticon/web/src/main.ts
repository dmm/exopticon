import {
  enableProdMode,
  provideZoneChangeDetection,
  isDevMode,
  importProvidersFrom,
} from "@angular/core";

import { environment } from "./environments/environment";
import {
  platformBrowser,
  BrowserModule,
  bootstrapApplication,
} from "@angular/platform-browser";
import {
  HTTP_INTERCEPTORS,
  provideHttpClient,
  withInterceptorsFromDi,
} from "@angular/common/http";
import { AuthInterceptor } from "./app/auth.interceptor";
import { APP_BASE_HREF } from "@angular/common";
import { CameraService } from "./app/camera.service";
import { TokenService } from "./app/token.service";
import { FormsModule, ReactiveFormsModule } from "@angular/forms";
import { AppRoutingModule } from "./app/app-routing.module";
import { IntersectionObserverModule } from "@ng-web-apis/intersection-observer";
import { ServiceWorkerModule } from "@angular/service-worker";
import { AppComponent } from "./app/app.component";

if (environment.production) {
  enableProdMode();
}

bootstrapApplication(AppComponent, {
  providers: [
    importProvidersFrom(
      BrowserModule,
      FormsModule,
      AppRoutingModule,
      IntersectionObserverModule,
      ReactiveFormsModule,
      ServiceWorkerModule.register("ngsw-worker.js", {
        enabled: !isDevMode(),
        registrationStrategy: "registerImmediately",
      }),
    ),
    { provide: HTTP_INTERCEPTORS, useClass: AuthInterceptor, multi: true },
    { provide: APP_BASE_HREF, useValue: "/" },
    CameraService,
    TokenService,
    provideHttpClient(withInterceptorsFromDi()),
  ],
}).catch((err) => console.error(err));
