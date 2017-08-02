defmodule Exopticon.CapturePort do
  use GenServer

  ### Client API
  def start_link(state, opts \\ []) do
    GenServer.start_link(__MODULE__, state, opts)
  end


  ### Server callbacks
  def init({id, url, fps, video_dir}) do
    Process.flag(:trap_exit, true)
    port = Port.open({:spawn, "apps/exopticon/src/captureserver #{url} #{fps} #{video_dir} /tmp/shot.jpg"},
      [:binary, { :packet, 4 }])
#    port = Port.open({:spawn, "pwd"}, [:binary, {:packet, 4}])
    {:ok, %{port: port}}
  end

  def handle_call(arg, _from, _names) do

  end

  def handle_cast(arg, _names) do

  end

  # Handle messages from port
  def handle_info({ port, { :data, msg } },  _names) do
     %{frameJpeg: %Bson.Bin{bin: dec }} = Bson.decode(msg)
     ExopticonWeb.Endpoint.broadcast!("test:lobby", "jpg", %{ frameJpeg: dec })
     {:noreply, 3}
  end

  def terminate(_reason, %{port: port}) do
    Port.close(port)
    :normal
  end

end
