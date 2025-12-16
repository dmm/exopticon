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

import { Component, OnInit } from "@angular/core";
import { Router } from "@angular/router";
import { LocalDate } from "@js-joda/core";
import { Observable } from "rxjs";
import { Token, TokenService } from "../token.service";

@Component({
    selector: "app-token-list",
    templateUrl: "./token-list.component.html",
    styleUrls: ["./token-list.component.css"],
    standalone: false
})
export class TokenListComponent implements OnInit {
  tokenName: string;
  tokenExpiration: string;
  error: string;
  tokens$: Observable<Token[]>;
  newToken: string;
  minDate: LocalDate = LocalDate.now().plusDays(1);

  constructor(
    private router: Router,
    private tokenService: TokenService,
  ) {}

  ngOnInit(): void {
    this.tokens$ = this.tokenService.getTokens();
    this.newToken = null;
  }

  onDelete(tokenId: number) {
    this.tokenService
      .deleteToken(tokenId)
      .toPromise()
      .then(() => {
        this.newToken = null;
        this.tokens$ = this.tokenService.getTokens();
      });
  }

  onSubmit(): void {
    console.log(`Token Name: ${this.tokenName}`);
    console.log(`Token Expiration: ${this.tokenExpiration}`);
    this.error = null;
    let expiration = `${this.tokenExpiration}T00:00:00Z`;
    this.tokenService
      .setToken(this.tokenName, expiration)
      .toPromise()
      .then((tokenKey) => {
        this.newToken = tokenKey;
        this.tokens$ = this.tokenService.getTokens();
      })
      .catch((err) => {
        console.log(`Error creating token: ${err}`);
        this.error = "Error creating token!";
      });
  }

  copyToken(): void {
    navigator.clipboard.writeText(this.newToken);
  }
}
