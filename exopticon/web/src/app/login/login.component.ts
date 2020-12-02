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

import { Component, OnInit } from "@angular/core";
import { FormControl, FormGroup } from "@angular/forms";
import { ActivatedRoute, Router } from "@angular/router";
import { combineLatest } from "rxjs";
import { map } from "rxjs/operators";
import { AuthService } from "../auth.service";

@Component({
  selector: "app-login",
  templateUrl: "./login.component.html",
  styleUrls: ["./login.component.css"],
})
export class LoginComponent implements OnInit {
  loginForm = new FormGroup({
    username: new FormControl(""),
    password: new FormControl(""),
  });

  constructor(
    private route: ActivatedRoute,
    private router: Router,
    private authService: AuthService
  ) {}

  ngOnInit(): void {}

  tryLogin() {
    let redirectPath = this.route.queryParamMap.pipe(
      map((params) => {
        if (params.has("redirect_path")) {
          return params.get("redirect_path");
        } else {
          return "/";
        }
      })
    );

    let username = this.loginForm.controls.username.value;
    let password = this.loginForm.controls.password.value;
    console.log(`${username} and ${password}`);
    combineLatest([
      redirectPath,
      this.authService.login(username, password),
    ]).subscribe(([path, outcome]) => {
      if (outcome) {
        this.router.navigate([path]);
      }
    });
  }
}
