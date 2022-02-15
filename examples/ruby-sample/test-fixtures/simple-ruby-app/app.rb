require 'socket'

server = TCPServer.new(12345)

loop do
  socket = server.accept
  socket.print(socket.gets.delete("\n").reverse)
  socket.close
end
