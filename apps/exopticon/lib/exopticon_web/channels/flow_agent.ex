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

defmodule ExopticonWeb.FlowAgent do
  @moduledoc """
  Provides flow control agent for video streams
  """
  use Agent

  def start_link() do
    Agent.start_link(fn -> Map.new() end, name: __MODULE__)
  end

  def reset(socket_id) do
    Agent.update(__MODULE__, fn map -> Map.delete(map, socket_id) end)
    :ok
  end

  def try_send(socket_id) do
    flow_state =
      Agent.get(__MODULE__, fn map ->
        Map.get(map, socket_id, %{cur_live: 0, max_live: 1})
      end)

    current_live = Map.get(flow_state, :cur_live)
    max_live = Map.get(flow_state, :max_live)

    ready = current_live < max_live

    current_live =
      if ready do
        current_live + 1
      else
        current_live
      end

    Agent.update(__MODULE__, &Map.put(&1, socket_id, Map.put(flow_state, :cur_live, current_live)))

    ready
  end

  def ack(socket_id, rtt) do
    flow_state =
      Agent.get(__MODULE__, fn map ->
        Map.get(map, socket_id)
      end)

    cur_live = Map.get(flow_state, :cur_live) - 1
    max_live = Map.get(flow_state, :max_live)
    old_rtt = Map.get(flow_state, :rtt, rtt)

    new_rtt = (rtt + old_rtt) / 2

    max_live =
      if new_rtt > 2 * old_rtt do
        Enum.max([div(max_live, 2), 1])
      else
        Enum.min([max_live + 1, 10])
      end

    Agent.update(
      __MODULE__,
      &Map.put(&1, socket_id, %{cur_live: cur_live, max_live: max_live, rtt: new_rtt})
    )
  end
end
