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

defmodule Exopticon.EqualQueue do
  @moduledoc """

  Provides a queue which takes an id tag and only allows a single
  element with each tag. If a new element is enqueued with a given tag
  it replaces the old one.

  """

  def new() do
    {:queue.new, %{}}
  end

  def push({q, m}, key, value) do
    case Map.has_key?(m, key) do
      true -> {q, Map.put(m, key, value)}
      false -> {:queue.in(key, q), Map.put(m, key, value)}
    end
  end

  def pop({_, m} = q) when m == %{} do
    {{nil, nil}, q}
  end

  def pop({q, m}) do
      {{:value, key}, q2} = :queue.out(q)
      item = Map.get(m, key)
      m2 = Map.delete(m, key)

      {{key, item}, {q2, m2}}
  end

  def len({_, m}) do
    m |> Map.keys() |> Enum.count
  end
end
