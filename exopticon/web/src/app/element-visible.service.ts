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

import { ElementRef, Inject, Injectable, DOCUMENT } from "@angular/core";
import { combineLatest, concat, defer, fromEvent, Observable, of } from "rxjs";
import { distinctUntilChanged, mergeMap, map } from "rxjs/operators";

@Injectable({
  providedIn: "root",
})
export class ElementVisibleService {
  private pageVisible$: Observable<boolean>;

  constructor(@Inject(DOCUMENT) document: Document) {
    this.pageVisible$ = concat(
      defer(() => of(!document.hidden)),
      fromEvent(document, "visibilitychange").pipe(
        map((e) => !document.hidden),
      ),
    );
  }

  elementVisible(element: ElementRef<Element>): Observable<boolean> {
    const elementVisible$ = new Observable<IntersectionObserverEntry[]>(
      (observer) => {
        const intersectionObserver = new IntersectionObserver((entries) => {
          observer.next(entries);
        });

        intersectionObserver.observe(element.nativeElement);

        return () => {
          intersectionObserver.disconnect();
        };
      },
    ).pipe(
      mergeMap((entries: IntersectionObserverEntry[]) => entries),
      map((entry: IntersectionObserverEntry) => entry.isIntersecting),
      distinctUntilChanged(),
    );

    const elementInViewport$ = combineLatest([
      this.pageVisible$,
      elementVisible$,
    ]).pipe(
      map(([pageVisible, elementVisible]) => pageVisible && elementVisible),
      distinctUntilChanged(),
    );

    return elementInViewport$;
  }
}
