defmodule Exopticon.CapturePort do
  @moduledoc """
  Provides port implementation that captures, saves, and streams video from a live stream.
  """
  use GenServer

  import Ecto.Query

  alias Ecto.Changeset
  alias Exopticon.Repo
  alias Exopticon.Video.VideoUnit

  require Logger

  ### Client API
  def start_link({id, _, _, _} = state, opts \\ []) do
    GenServer.start_link(__MODULE__, state, name: via_tuple(id))
  end

  def child_spec(camera) do
    args = {
      camera.id,
      camera.rtsp_url,
      camera.fps,
      camera.camera_group.storage_path
    }

    %{
      id: camera.id,
      start: {Exopticon.CapturePort, :start_link, [args]},
      restart: :permanent,
      type: :worker
    }
  end

  ### Server callbacks
  defp get_monotonic_index_for_id(id) do
    index =
      Repo.one(
        from(
          vu in "video_units",
          select: max(vu.monotonic_index),
          where: vu.camera_id == ^id
        )
      )

    if index == nil do
      1
    else
      index + 1
    end
  end

  def start_port({id, url, fps, video_dir}) do
    Logger.metadata(camera_id: id)

    storage_path =
      video_dir
      |> Path.expand()
      |> Path.join(Integer.to_string(id))

    File.mkdir_p!(storage_path)

    Port.open(
      {
        :spawn,
        "apps/exopticon/lib/exopticon/captureworker '#{url}' #{fps} #{storage_path} /tmp/shot.jpg"
      },
      [:binary, {:packet, 4}, :exit_status]
    )
  end

  def init({id, url, fps, video_dir} = port_args) do
    monotonic_index = get_monotonic_index_for_id(id)
    Logger.info("starting port #{id} #{url}")
    port = start_port({id, url, fps, video_dir})

    {
      :ok,
      %{
        port: port,
        id: id,
        monotonic_index: monotonic_index,
        video_unit_id: 0,
        frame_index: -1,
        port_args: port_args
      }
    }
  end

  # Handle messages from port
  def handle_port_message({%{"jpegFrame" => dec, "pts" => pts}, %{
                              port: port,
                              id: id,
                              monotonic_index: monotonic_index,
                              video_unit_id: video_unit_id,
                              frame_index: frame_index,
                              port_args: port_args
                           }}) do
    ExopticonWeb.Endpoint.broadcast!(
      "camera:stream",
      "jpg",
      %{
        cameraId: id,
        frameJpeg: Msgpax.Bin.new(dec),
        pts: pts,
        res: "hd"
      })
    %{
      port: port,
      id: id,
      monotonic_index: monotonic_index,
      video_unit_id: video_unit_id,
      frame_index: frame_index + 1,
      port_args: port_args
    }
  end

  def handle_port_message({%{"jpegFrameScaled" => dec, "pts" => pts, "height" => _height}, %{
                              port: port,
                              id: id,
                              monotonic_index: monotonic_index,
                              video_unit_id: video_unit_id,
                              frame_index: frame_index,
                              port_args: port_args
                           } = state}) do
    ExopticonWeb.Endpoint.broadcast!("camera:stream", "jpg",
      %{
        cameraId: id,
        frameJpeg: Msgpax.Bin.new(dec),
        pts: pts,
        res: "sd"
      })
    state
  end

  def handle_port_message({
    %{"filename" => filename, "beginTime" => beginTime},
    %{
      port: port,
      id: id,
      monotonic_index: monotonic_index,
      video_unit_id: video_unit_id,
      frame_index: frame_index,
      port_args: port_args
    }}) do
    {:ok, start_time, _} = DateTime.from_iso8601(beginTime)
    monotonic_start = System.monotonic_time(:microsecond)

    {:ok , video_unit} =
      %Exopticon.Video.VideoUnit{
        camera_id: id,
        begin_time: start_time,
        end_time: nil,
        begin_monotonic: monotonic_start,
        monotonic_index: monotonic_index,
        files: [
          %Exopticon.Video.File{filename: filename, size: 0}
        ]
      }
      |> Repo.insert()

    %{port: port, id: id, monotonic_index: monotonic_index, video_unit_id: video_unit.id, frame_index: 0, port_args: port_args}
  end

  def handle_port_message({%{"filename" => filename, "endTime" => endTime}, %{
                              port: port,
                              id: id,
                              monotonic_index: monotonic_index,
                              video_unit_id: video_unit_id,
                              frame_index: frame_index,
                              port_args: port_args
                           }}) do
    monotonic_stop = System.monotonic_time(:microsecond)
    {:ok, end_time, _} = DateTime.from_iso8601(endTime)
    video_unit = VideoUnit |> Repo.get(video_unit_id, preload: [:files]) |> Repo.preload(:files)
    file = video_unit.files |> List.first

    %{size: size} = File.stat!(filename)

    file
    |> Changeset.change(size: size)
    |> Repo.update()

    video_unit
    |> Changeset.change(end_time: end_time)
    |> Changeset.change(end_monotonic: monotonic_stop)
    |> Repo.update()

    %{port: port, id: id, monotonic_index: monotonic_index, video_unit_id: 0, frame_index: -1, port_args: port_args}
  end

  def handle_port_message({%{"type" => "log", "level" => level, "message" => message}, state}) do
    Logger.debug(message)
    state
  end

  def handle_info({port, {:data, msg}}, %{
        port: port,
        id: id,
        monotonic_index: monotonic_index,
        video_unit_id: video_unit_id,
        frame_index: frame_index,
        port_args: port_args
      } = state) do

    ret = {Msgpax.unpack!(msg), state} |> handle_port_message

    {:noreply, ret}
  end

  def handle_info({:EXIT, _port, reason}, state) do
    Logger.info("Capture port stopped! The reason is: #{Atom.to_string(reason)}")
    {:stop, reason, state}
  end

  def handle_info({_port, {:exit_status, status}}, %{
        port: port,
        id: id,
        monotonic_index: monotonic_index,
        video_unit_id: video_unit_id,
        frame_index: frame_index,
        port_args: port_args
      }) do
    # sleep for five seconds and then restart the port
    :timer.sleep(5000)
    new_port = start_port(port_args)
    {:noreply, %{port: new_port, id: id, monotonic_index: monotonic_index, video_unit_id: 0, frame_index: -1, port_args: port_args}}
  end

  def terminate(_reason, %{id: _, port: port}) do
    Logger.info("Capture port terminated!")
    if Port.info(port) != nil do
    end

    :normal
  end

  defp via_tuple(topic) do
    {:via, Registry, {Registry.CameraRegistry, topic}}
  end
end
