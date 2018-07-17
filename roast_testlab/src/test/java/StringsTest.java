import org.junit.Test;

import static org.junit.Assert.assertEquals;

public class StringsTest {

    @Test
    public void helloWorld() {
        assertEquals("Hello, World!", Strings.helloWorld());
    }

    @Test
    public void reverse() {
        assertEquals("tsaor", Strings.reverse("roast"));
    }

    @Test
    public void countChars() {
        assertEquals(5, Strings.countChars("roast"));
    }

}