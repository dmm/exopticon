defmodule Exopticon.CapturePort do
  use GenServer

  import Exopticon.Repo
  import Exopticon.Video.File
  import Ecto.Query

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
      shutdown: 5000,
      type: :worker
      }
  end


  ### Server callbacks
    defp get_monotonic_index_for_id(id) do
    index = Exopticon.Repo.one(from f in "files",
      select: max(f.monotonic_index),
      where: f.camera_id == ^id)
    if index == nil do
      1
    else
      index + 1
    end
  end

  def init({id, url, fps, video_dir}) do
    storage_path = video_dir
    |> Path.expand
    |> Path.join(Integer.to_string(id))
    File.mkdir_p!(storage_path)
    port = Port.open({:spawn, "apps/exopticon/src/captureserver #{url} #{fps} #{storage_path} /tmp/shot.jpg"},
      [:binary, { :packet, 4 }, :exit_status])
    monotonic_index = get_monotonic_index_for_id(id)
    {:ok, %{port: port, id: id, monotonic_index: monotonic_index}}
  end

#  def handle_call(arg, _from, _names) do

 # end

  #def handle_cast(arg, _names) do

  #end

  # Handle messages from port
  def handle_info({ port, { :data, msg } }, %{port: port, id: id, monotonic_index: monotonic_index}) do
    {Bson.decode(msg), id, monotonic_index}
    |> handle_message

    {:noreply, %{port: port, id: id, monotonic_index: monotonic_index}}
  end

  def handle_message({%{frameJpeg: %Bson.Bin{bin: dec}, pts: pts}, id, _}) do
    ExopticonWeb.Endpoint.broadcast!("camera:"<>Integer.to_string(id), "jpg", %{ frameJpeg: dec, pts: pts})
  end

  def handle_message({%{type: "newFile", filename: filename, beginTime: beginTime}, id, monotonic_index}) do
    IO.puts "Got new file from " <> Integer.to_string(id) <> "! " <> filename <> " at " <> beginTime
    {:ok, start_time, _} = DateTime.from_iso8601(beginTime)
    monotonic_start = System.monotonic_time(:microsecond)
    {:ok, time} = Exopticon.Tsrange.cast([start_time, nil])
    %Exopticon.Video.File{filename: filename, camera_id: id, time: time, begin_monotonic: monotonic_start,
                          monotonic_index: monotonic_index}

    |> Exopticon.Repo.insert

  end

  def handle_message({%{type: "endFile", filename: filename, endTime: endTime}, id, _}) do
    monotonic_stop = System.monotonic_time(:microsecond)
    IO.puts "Got end file from " <> Integer.to_string(id) <> "! " <> filename <> " at " <> endTime
    {:ok, end_time, _} = DateTime.from_iso8601(endTime)
    file = Exopticon.Repo.get_by(Exopticon.Video.File, filename: filename)
    [start_time, _] = file.time
    {:ok, time} = Exopticon.Tsrange.cast([start_time, end_time])

    file
    |> Ecto.Changeset.change(time: time)
    |> Ecto.Changeset.change(end_monotonic: monotonic_stop)
    |> Exopticon.Repo.update
  end

  def handle_info({:EXIT, _port, reason}, state) do
    IO.puts "Capture port stoppped! The reason is: " <> Atom.to_string(reason)
    {:stop, reason, state}
  end

  def handle_info({_port, {:exit_status, status}}, state) do
    IO.puts "Exit Status: " <> Integer.to_string(status)
    {:stop, :normal, state}
  end

  def terminate(_reason, %{id: _, port: port}) do
    if Port.info(port) != nil do
      Port.close(port)
    end
    :normal
  end

end
