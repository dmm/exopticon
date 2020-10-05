import { DOCUMENT } from "@angular/common";
import { ElementRef, Inject, Injectable } from "@angular/core";
import { combineLatest, concat, defer, fromEvent, Observable, of } from "rxjs";
import { distinctUntilChanged, flatMap, map } from "rxjs/operators";

@Injectable({
  providedIn: "root",
})
export class ElementVisibleService {
  private pageVisible$: Observable<boolean>;

  constructor(@Inject(DOCUMENT) document: any) {
    console.log("got document! " + document);
    this.pageVisible$ = concat(
      defer(() => of(!document.hidden)),
      fromEvent(document, "visibilitychange").pipe(map((e) => !document.hidden))
    );
  }

  elementVisible(element: ElementRef): Observable<boolean> {
    const elementVisible$ = Observable.create((observer) => {
      const intersectionObserver = new IntersectionObserver((entries) => {
        observer.next(entries);
      });

      console.log(element);
      intersectionObserver.observe(element.nativeElement);

      return () => {
        intersectionObserver.disconnect();
      };
    }).pipe(
      flatMap((entries: IntersectionObserverEntry[]) => entries),
      map((entry: IntersectionObserverEntry) => entry.isIntersecting),
      distinctUntilChanged()
    );

    const elementInViewport$ = combineLatest(
      this.pageVisible$,
      elementVisible$,
      (pageVisible, elementVisible: boolean) => pageVisible && elementVisible
    ).pipe(distinctUntilChanged());

    return elementInViewport$;
  }
}
