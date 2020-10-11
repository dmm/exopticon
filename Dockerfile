FROM dmattli/exopticon-build:devel AS prod-build

WORKDIR /exopticon

USER exopticon:exopticon

COPY --chown=exopticon:exopticon . ./

RUN cargo make --profile release build-release

FROM dmattli/exopticon-build:runtime

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/exopticon .

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/assets/workers ./workers

ENV EXOPTICONWORKERS=/exopticon/workers/
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=/exopticon:$PATH

USER exopticon

ENTRYPOINT /exopticon/exopticon
