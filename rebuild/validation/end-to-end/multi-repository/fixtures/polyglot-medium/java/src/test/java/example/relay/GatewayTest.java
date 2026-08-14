package example.relay;

public final class GatewayTest {
    public void routesRequest() {
        assert new Gateway().route(new Route("Ada", "en")).equals("Ada:en");
    }
}
