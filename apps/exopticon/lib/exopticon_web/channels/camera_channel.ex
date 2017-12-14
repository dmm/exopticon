defmodule ExopticonWeb.CameraChannel do
  use ExopticonWeb, :channel

  intercept(["jpg"])

  def join("camera:lobby", payload, socket) do
    if authorized?(payload) do
      {:ok, socket}
    else
      {:error, %{reason: "unauthorized"}}
    end
  end

  def join("camera:" <> _camera_id, _params, socket) do
    socket = assign(socket, :max_live, 10)
    socket = assign(socket, :cur_live, 0)
    {:ok, socket}
  end

  # Channels can be used in a request/response fashion
  # by sending replies to requests from the client
  def handle_in("ping", payload, socket) do
    {:reply, {:ok, payload}, socket}
  end

  def handle_in("ack", _payload, socket) do
    cur_live = socket.assigns[:cur_live]
    socket = assign(socket, :cur_live, cur_live - 1)
    {:noreply, socket}
  end

  # It is also common to receive messages from the client and
  # broadcast to everyone in the current topic (camera:lobby).
  def handle_in("shout", payload, socket) do
    broadcast(socket, "shout", payload)
    {:noreply, socket}
  end

  def handle_out("jpg", params, socket) do
    cur_live = socket.assigns[:cur_live]
    max_live = socket.assigns[:max_live]

    cond do
      cur_live < max_live ->
        push(socket, "jpg", params)
        socket = assign(socket, :cur_live, cur_live + 1)

      cur_live == max_live ->
        new_max = Enum.max([div(max_live, 2), 1])
        socket = assign(socket, :max_live, new_max)

      true ->
        # should never happen
        nil
    end

    {:noreply, socket}
  end

  # Add authorization logic here as required.
  defp authorized?(_payload) do
    true
  end
end
