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
