defmodule Exopticon.Tsrange do
  @behaviour Ecoto.Type

  def type, do: :tsrange

  def cast([lower, upper]) do
    case apply_func([lower, upper], &Ecto.DateTime.cast/1) do
      {:ok, [lower, upper]} -> {:ok, [lower, upper]}
      :error -> :error
    end
  end

  def cast(_), do: :error

  def load(%Postgrex.Range{lower: lower, upper: upper}) do
    apply_func([lower, upper], &Ecto.DateTime.load/1)
  end

  def dump([lower, upper]) do
    case apply_func([lower, upper], &Ecto.DateTime.dump/1) do
      {:ok, [lower, upper]} ->
        {:ok, %Postgrex.Range{lower: lower, upper: upper, upper_inclusive: false}}

      :error ->
        :error
    end
  end

  def dump(_), do: :error

  defp apply_func([lower, upper], fun) do
    lower = apply_func(lower, fun)

    upper = apply_func(upper, fun)

    if lower != :error and upper != :error do
      {:ok, [lower, upper]}
    else
      :error
    end
  end

  defp apply_func(nil, fun) do
    nil
  end

  defp apply_func(target, fun) do
    case fun.(target) do
      {:ok, target} -> target
      :error -> :error
    end
  end
end
