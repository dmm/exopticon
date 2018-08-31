# This file is a part of Exopticon, a free video surveillance tool. Visit
# https://exopticon.org for more information.
#
# Copyright (C) 2018 David Matthew Mattli
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

defmodule ExopticonWeb.UserSocket do
  use Phoenix.Socket

  import Exopticon.EqualQueue

  @max_age 2 * 7 * 24 * 60 * 60

  ## Channels
  channel("camera:*", ExopticonWeb.CameraChannel)
  channel("playback:*", ExopticonWeb.PlaybackChannel)

  ## Transports
  transport(
    :websocket,
    Phoenix.Transports.WebSocket,
    serializer: [
      {ExopticonWeb.Transports.MessagePackSerializer, "~> 2.0.0"}
    ]
  )

  # transport :longpoll, Phoenix.Transports.LongPoll

  # Socket params are passed from the client and can
  # be used to verify and authenticate a user. After
  # verification, you can put default assigns into
  # the socket that will be set for all channels, ie
  #
  #     {:ok, assign(socket, :user_id, verified_user_id)}
  #
  # To deny connection, return `:error`.
  #
  # See `Phoenix.Token` documentation for examples in
  # performing token verification on connect.
  def connect(%{"token" => token}, socket) do
    case Phoenix.Token.verify(socket, "user socket", token, max_age: @max_age) do
      {:ok, user_id} ->
        socket = assign(socket, :user_id, user_id)
        socket = assign(socket, :max_live, 1)
        socket = assign(socket, :cur_live, 0)
        socket = assign(socket, :rtt, 0)
        socket = assign(socket, :watch_camera, %{})
        socket = assign(socket, :hd_cameras, %{})
        socket = assign(socket, :window, {0, 0, 0})
        {:ok, socket}

      {:error, _reason} ->
        :error
    end
  end

  def connect(_params, _socket), do: :error

  # Socket id's are topics that allow you to identify all sockets for a given user:
  #
  #     def id(socket), do: "user_socket:#{socket.assigns.user_id}"
  #
  # Would allow you to broadcast a "disconnect" event and terminate
  # all active sockets and channels for a given user:
  #
  #     ExopticonWeb.Endpoint.broadcast("user_socket:#{user.id}", "disconnect", %{})
  #
  # Returning `nil` makes this socket anonymous.
  def id(socket), do: "users_socket:#{socket.assigns.user_id}"
end
