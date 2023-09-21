require 'socket'

port = ENV["PORT"]
port = 12345 if port.nil?

server = TCPServer.new(port)

loop do
  socket = server.accept
  socket.print(socket.gets.delete("\n").reverse)
  socket.close
end
