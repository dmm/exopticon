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
import { Component } from "@angular/core";
import { ActivatedRoute, Router } from "@angular/router";
import { Observable } from "rxjs";
import { AuthService } from "./auth.service";

export enum MenuState {
  None,
  Main,
  Armed,
}

@Component({
  selector: "app-root",
  templateUrl: "./app.component.html",
  styleUrls: ["./app.component.css"],
  standalone: false,
})
export class AppComponent {
  public menuStates = MenuState;
  title = "exopticon";
  fullscreen = false;
  menuState = MenuState.None;
  isLoggedIn$: Observable<boolean>;

  constructor(
    private route: ActivatedRoute,
    public router: Router,
    private authService: AuthService,
  ) {}

  ngOnInit() {
    this.route.queryParamMap.subscribe((params) => {
      if (params.has("fs")) {
        this.fullscreen = params.get("fs") === "true";
      } else {
        this.fullscreen = false;
      }
    });
    this.isLoggedIn$ = this.authService.isLoggedIn;
  }

  redirect() {
    this.router.navigateByUrl("/camera_panel");
  }

  handleClick(menu) {
    if (menu === "main" && this.menuState == MenuState.None) {
      this.menuState = MenuState.Main;
    } else if (menu === "main" && this.menuState == MenuState.Main) {
      this.menuState = MenuState.None;
    }
  }

  closeMenu() {
    this.menuState = MenuState.None;
  }
}
