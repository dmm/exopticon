defmodule Exvif.Util do
  @moduledoc false
  @user_agent [{"User-agent", "Exvif onvif client"}]
  @timeout 60

  def password_digest(username, password) do
    password_digest(username, password, 0)
  end

  def password_digest(_username, password, time_offset) do
    max = 0x100000000
    timestamp = Timex.now() |> Timex.shift(seconds: time_offset)
    {:ok, timestamp_buffer} = Timex.format(timestamp, "{ISO:Extended:Z}")

    nonce =
      <<:rand.uniform(max)::little-size(32)>> <>
        <<:rand.uniform(max)::little-size(32)>> <>
        <<:rand.uniform(max)::little-size(32)>> <> <<:rand.uniform(max)::little-size(32)>>

    digest = :crypto.hash(:sha, nonce <> timestamp_buffer <> password)

    %{
      password_digest: Base.encode64(digest),
      nonce: Base.encode64(nonce),
      timestamp: timestamp_buffer
    }
  end

  def security_block("", "") do
    ""
  end

  def security_block(username, password) do
    %{password_digest: password_digest, nonce: nonce, timestamp: timestamp} =
      password_digest(username, password)

    """
     <Security s:mustUnderstand="1" xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
    			<UsernameToken>
    			<Username>#{username}</Username>
          <Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">#{
      password_digest
    }</Password>
    			<Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">#{
      nonce
    }</Nonce>
    			<Created xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">#{
      timestamp
    }</Created>
    			</UsernameToken>
    </Security>
    """
  end

  def envelope_header(username, password) do
    ~s(
    <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://www.w3.org/2005/08/addressing">
    <s:Header>
      #{security_block(username, password)}
      </s:Header>
      <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
      )
  end

  def envelope_footer do
    """
    </s:Body></s:Envelope>
    """
  end

  def request(url, body) do
    headers =
      [
        {"Content-Type", "application/soap+xml"},
        {"Content-Length", byte_size(body)},
        {"charset", "utf-8"}
      ] ++ @user_agent

    case HTTPoison.post(url, body, headers, timeout: @timeout) do
      {:ok, response} -> {:ok, response}
      {:error, error} -> {:error, error}
    end
  end
end
