defmodule Exopticon.CapturePort do
  @moduledoc """
  Provides port implementation that captures, saves, and streams video from a live stream.
  """
  use GenServer

  import Ecto.Query

  alias Ecto.Changeset
  alias Exopticon.Repo

  ### Client API
  def start_link(state, opts \\ []) do
    GenServer.start_link(__MODULE__, state, opts)
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
          f in "files",
          select: max(f.monotonic_index),
          where: f.camera_id == ^id
        )
      )

    if index == nil do
      1
    else
      index + 1
    end
  end

  def start_port({id, url, fps, video_dir}) do
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

  def init({id, url, fps, video_dir}) do
    monotonic_index = get_monotonic_index_for_id(id)
    port = start_port({id, url, fps, video_dir})

    {
      :ok,
      %{
        port: port,
        id: id,
        monotonic_index: monotonic_index,
        port_args: {id, url, fps, video_dir}
      }
    }
  end

  # Handle messages from port
  def handle_port_message({%{"jpegFrame" => _dec, "pts" => _pts}, _id, _}) do
    #    ExopticonWeb.Endpoint.broadcast!("camera:stream", "jpg", %{
    #      cameraId: id,
    #      frameJpeg: Msgpax.Bin.new(dec),
    #      pts: pts
    #    })
  end

  def handle_port_message({%{"jpegFrameScaled" => dec, "pts" => pts, "height" => _height}, id, _}) do
    ExopticonWeb.Endpoint.broadcast!("camera:stream", "jpg", %{
      cameraId: id,
      frameJpeg: Msgpax.Bin.new(dec),
      pts: pts
    })
  end

  def handle_port_message({
        %{"filename" => filename, "beginTime" => beginTime},
        id,
        monotonic_index
      }) do
    {:ok, start_time, _} = DateTime.from_iso8601(beginTime)
    monotonic_start = System.monotonic_time(:microsecond)

    %Exopticon.Video.File{
      filename: filename,
      camera_id: id,
      begin_time: start_time,
      end_time: nil,
      begin_monotonic: monotonic_start,
      monotonic_index: monotonic_index
    }
    |> Repo.insert()
  end

  def handle_port_message({%{"filename" => filename, "endTime" => endTime}, _id, _}) do
    monotonic_stop = System.monotonic_time(:microsecond)
    {:ok, end_time, _} = DateTime.from_iso8601(endTime)
    file = Repo.get_by(Exopticon.Video.File, filename: filename)
    %{size: size} = File.stat!(filename)

    file
    |> Changeset.change(end_time: end_time)
    |> Changeset.change(size: size)
    |> Changeset.change(end_monotonic: monotonic_stop)
    |> Repo.update()
  end

  def handle_info({port, {:data, msg}}, %{
        port: port,
        id: id,
        monotonic_index: monotonic_index,
        port_args: port_args
      }) do
    {Msgpax.unpack!(msg), id, monotonic_index}
    |> handle_port_message

    {:noreply, %{port: port, id: id, monotonic_index: monotonic_index, port_args: port_args}}
  end

  def handle_info({:EXIT, _port, reason}, state) do
    IO.puts("Capture port stoppped! The reason is: " <> Atom.to_string(reason))
    {:stop, reason, state}
  end

  def handle_info({_port, {:exit_status, status}}, %{
        port: _port2,
        id: id,
        monotonic_index: monotonic_index,
        port_args: port_args
      }) do
    # sleep for five seconds and then restart the port
    :timer.sleep(5000)
    new_port = start_port(port_args)
    {:noreply, %{port: new_port, id: id, monotonic_index: monotonic_index, port_args: port_args}}
  end

  def terminate(_reason, %{id: _, port: port}) do
    if Port.info(port) != nil do
    end

    :normal
  end
end
