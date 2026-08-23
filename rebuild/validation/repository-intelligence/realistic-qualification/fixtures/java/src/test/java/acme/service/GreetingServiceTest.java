package acme.service;

public class GreetingServiceTest {
    @Test
    public void rendersNormalizedName() {
        new GreetingService().render(" Ada ");
    }
}
